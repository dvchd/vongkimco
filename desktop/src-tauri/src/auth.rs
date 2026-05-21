//! Desktop-side auth: browser-OAuth login flow + token lifecycle.
//!
//! All state that survives across app restarts lives in two places:
//!   - OS keyring: the refresh token (under service "vongkimco-desktop",
//!     account "refresh-token"). Never written to plain files.
//!   - In-memory `AuthStore`: access token + expiry + user info. Never
//!     persisted; rehydrated at boot via `try_restore_session`.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::state::{AppState, UserInfo};

const KEYRING_SERVICE: &str = "vongkimco-desktop";
const KEYRING_ACCOUNT: &str = "refresh-token";

/// Refresh `expires_at - now` below this many seconds and we treat the
/// access token as effectively expired. Five minutes is generous enough to
/// avoid mid-request token expiry.
pub const REFRESH_LEEWAY_SECS: i64 = 5 * 60;

// ---------- Fingerprint ----------

/// Stable per-machine identifier derived from the OS machine-id plus
/// hostname. Same machine → same fingerprint across reboots / app
/// reinstalls; the server uses this to bind refresh tokens to the device.
pub fn device_fingerprint() -> String {
    let machine = machine_uid::get().unwrap_or_else(|_| "unknown-machine".into());
    let host = hostname_or_default();
    let os = std::env::consts::OS;
    let mut h = Sha256::new();
    h.update(b"vongkimco-v1");
    h.update(machine.as_bytes());
    h.update(b"|");
    h.update(host.as_bytes());
    h.update(b"|");
    h.update(os.as_bytes());
    hex::encode(h.finalize())
}

pub fn hostname_or_default() -> String {
    if let Ok(h) = std::env::var("COMPUTERNAME") {
        return h;
    }
    if let Ok(h) = std::env::var("HOSTNAME") {
        return h;
    }
    "desktop".to_string()
}

pub fn random_state() -> String {
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    hex::encode(buf)
}

// ---------- Keyring ----------

pub fn save_refresh_token(token: &str) -> Result<()> {
    let entry =
        keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).context("open keyring entry")?;
    entry.set_password(token).context("write keyring")?;
    Ok(())
}

pub fn load_refresh_token() -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .ok()?
        .get_password()
        .ok()
}

pub fn clear_refresh_token() {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT) {
        let _ = entry.delete_credential();
    }
}

// ---------- HTTP helpers ----------

fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .context("build http client")
}

// ---------- Login flow ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingLogin {
    pub flow_id: String,
    pub state: String,
    pub started_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartLoginResp {
    pub flow_id: String,
    pub auth_url: String,
}

pub async fn start_login(state: &AppState) -> Result<StartLoginResp> {
    let server = state.server_url();
    let fingerprint = device_fingerprint();
    let host = hostname_or_default();
    let os = std::env::consts::OS.to_string();
    let csrf = random_state();

    let body = serde_json::json!({
        "device_fingerprint": fingerprint,
        "device_name": host,
        "os": os,
        "state": csrf,
    });

    let client = http_client()?;
    let resp = client
        .post(format!(
            "{}/api/v1/auth/desktop/start",
            server.trim_end_matches('/')
        ))
        .json(&body)
        .send()
        .await
        .context("post auth/desktop/start")?;
    if !resp.status().is_success() {
        return Err(anyhow!("HTTP {}", resp.status()));
    }
    let j: Value = resp.json().await.context("parse start response")?;
    let flow_id = j["flow_id"].as_str().unwrap_or_default().to_string();
    let auth_url = j["auth_url"].as_str().unwrap_or_default().to_string();
    if flow_id.is_empty() || auth_url.is_empty() {
        return Err(anyhow!("malformed start response"));
    }

    let now = chrono::Utc::now().timestamp();
    *state.pending_login.write() = Some(PendingLogin {
        flow_id: flow_id.clone(),
        state: csrf,
        started_at: now,
    });

    Ok(StartLoginResp { flow_id, auth_url })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PollOutcome {
    Pending,
    Expired,
    DeviceLimitExceeded,
    NotMember,
    Completed {
        access_token: String,
        refresh_token: String,
        user: UserInfo,
        #[serde(default)]
        subscription: Option<Value>,
    },
}

pub async fn poll_login(state: &AppState) -> Result<PollOutcome> {
    let pending = state.pending_login.read().clone();
    let Some(p) = pending else {
        return Err(anyhow!("no pending login"));
    };
    let server = state.server_url();
    let client = http_client()?;
    let resp = client
        .get(format!(
            "{}/api/v1/auth/desktop/poll/{}",
            server.trim_end_matches('/'),
            p.flow_id
        ))
        .send()
        .await
        .context("poll desktop flow")?;
    if !resp.status().is_success() {
        return Err(anyhow!("HTTP {}", resp.status()));
    }
    let outcome: PollOutcome = resp.json().await.context("parse poll response")?;

    if let PollOutcome::Completed {
        access_token,
        refresh_token,
        user,
        ..
    } = &outcome
    {
        save_refresh_token(refresh_token).context("save refresh token")?;
        let expires_at = chrono::Utc::now().timestamp() + crate::auth::ACCESS_TOKEN_TTL_HINT;
        {
            let mut a = state.auth.write();
            a.access_token = Some(access_token.clone());
            a.access_expires_at = expires_at;
            a.user = Some(user.clone());
        }
        *state.pending_login.write() = None;
    } else if matches!(
        outcome,
        PollOutcome::Expired | PollOutcome::DeviceLimitExceeded | PollOutcome::NotMember
    ) {
        *state.pending_login.write() = None;
    }

    Ok(outcome)
}

pub fn cancel_login(state: &AppState) {
    *state.pending_login.write() = None;
}

// ---------- Token refresh ----------

/// We don't know exactly when the access token expires (the server's clock,
/// not ours, is authoritative). We use the issuer's documented 1-hour
/// lifetime, minus 60s for clock skew, as the hint. /auth/refresh returns
/// `expires_at` precisely so subsequent refreshes don't rely on the hint.
pub const ACCESS_TOKEN_TTL_HINT: i64 = 3600 - 60;

#[derive(Debug, Deserialize)]
struct RefreshResp {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
}

/// Exchange the keyring's refresh token for a fresh access token. Returns
/// the access token. On HTTP 401 the keyring is cleared (the server has
/// explicitly rejected the token); on every other failure the keyring
/// stays put so a flaky network doesn't force the user to re-login.
pub async fn refresh_tokens(state: &AppState) -> Result<String> {
    let refresh = load_refresh_token().ok_or_else(|| anyhow!("no refresh token"))?;
    let fingerprint = device_fingerprint();
    let server = state.server_url();
    let client = http_client()?;
    let resp = client
        .post(format!(
            "{}/api/v1/auth/refresh",
            server.trim_end_matches('/')
        ))
        .json(&serde_json::json!({
            "refresh_token": refresh,
            "device_fingerprint": fingerprint,
        }))
        .send()
        .await
        .context("post auth/refresh")?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        clear_refresh_token();
        let mut a = state.auth.write();
        a.access_token = None;
        a.access_expires_at = 0;
        a.user = None;
        return Err(anyhow!("token_rejected"));
    }
    if !status.is_success() {
        return Err(anyhow!("refresh HTTP {}", status));
    }

    let body: RefreshResp = resp.json().await.context("parse refresh response")?;
    save_refresh_token(&body.refresh_token).context("save refresh token")?;
    {
        let mut a = state.auth.write();
        a.access_token = Some(body.access_token.clone());
        a.access_expires_at = body.expires_at;
    }
    Ok(body.access_token)
}

/// Return a valid access token, refreshing first if the current one is
/// missing or about to expire.
pub async fn ensure_fresh_token(state: &AppState) -> Result<String> {
    let now = chrono::Utc::now().timestamp();
    let (token, exp) = {
        let a = state.auth.read();
        (a.access_token.clone(), a.access_expires_at)
    };
    if let Some(t) = token {
        if exp - now > REFRESH_LEEWAY_SECS {
            return Ok(t);
        }
    }
    refresh_tokens(state).await
}

// ---------- Boot: restore session ----------

#[derive(Debug, Deserialize)]
struct VerifyResp {
    valid: bool,
    user: UserInfo,
}

/// Called once at startup. If we have a refresh token in the keyring, try
/// to mint a fresh access token and load user info. Returns true if the
/// session was restored. On network failure returns false but keeps the
/// keyring untouched (the user is "logged in but offline").
pub async fn try_restore_session(state: &AppState) -> bool {
    if load_refresh_token().is_none() {
        return false;
    }

    let token = match refresh_tokens(state).await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("restore_session: refresh failed: {e}");
            return false;
        }
    };

    let server = state.server_url();
    let client = match http_client() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let resp = match client
        .get(format!(
            "{}/api/v1/auth/verify",
            server.trim_end_matches('/')
        ))
        .bearer_auth(&token)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("restore_session: verify network failure: {e} (keeping token)");
            return false;
        }
    };
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        clear_refresh_token();
        let mut a = state.auth.write();
        a.access_token = None;
        a.access_expires_at = 0;
        a.user = None;
        return false;
    }
    if !resp.status().is_success() {
        return false;
    }
    let body: VerifyResp = match resp.json().await {
        Ok(b) => b,
        Err(_) => return false,
    };
    if !body.valid {
        return false;
    }
    {
        let mut a = state.auth.write();
        a.user = Some(body.user);
    }
    true
}

// ---------- Logout ----------

pub async fn logout(state: &AppState) -> Result<()> {
    clear_refresh_token();
    let mut a = state.auth.write();
    a.access_token = None;
    a.access_expires_at = 0;
    a.user = None;
    Ok(())
}
