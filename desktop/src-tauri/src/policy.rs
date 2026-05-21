//! Server-controlled data-collection policy. Fetched from
//! `GET /api/v1/config`, cached on disk so the app keeps working offline,
//! and re-polled every `refresh_interval_secs`. All knobs live here so
//! there's a single source of truth at runtime — `monitor.rs` and
//! `screenshot.rs` read from this, never from local `Settings`.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::state::AppState;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Policy {
    pub capture_screenshots: bool,
    pub screenshot_interval_secs: u64,
    pub activity_sample_interval_secs: u64,
    pub app_snapshot_interval_secs: u64,
    pub idle_threshold_secs: u64,
    pub screenshot_quality: u32,
    pub screenshot_max_width: u32,
    pub refresh_interval_secs: u64,
    /// Server-assigned version (the row's updated_at). Used purely for
    /// display — clients re-apply on every fetch, no diffing needed.
    #[serde(default)]
    pub version: String,
    /// Whether the current value came from the server (vs. the built-in
    /// defaults). UI uses this to decide whether to show the "managed by
    /// admin" badge.
    #[serde(default)]
    pub from_server: bool,
}

impl Default for Policy {
    fn default() -> Self {
        // Mirror the server-side migration defaults so an offline first run
        // behaves the same as a freshly-installed server.
        Self {
            capture_screenshots: true,
            screenshot_interval_secs: 180,
            activity_sample_interval_secs: 30,
            app_snapshot_interval_secs: 60,
            idle_threshold_secs: 120,
            screenshot_quality: 50,
            screenshot_max_width: 1280,
            refresh_interval_secs: 300,
            version: String::new(),
            from_server: false,
        }
    }
}

const POLICY_FILE: &str = "policy.json";

pub async fn load_from_disk(state: &AppState) -> Policy {
    let path = state.data_dir.join(POLICY_FILE);
    if !path.exists() {
        return Policy::default();
    }
    match tokio::fs::read_to_string(&path).await {
        Ok(txt) => serde_json::from_str(&txt).unwrap_or_default(),
        Err(_) => Policy::default(),
    }
}

async fn save_to_disk(state: &AppState, p: &Policy) -> Result<()> {
    let txt = serde_json::to_string_pretty(p)?;
    tokio::fs::write(state.data_dir.join(POLICY_FILE), txt)
        .await
        .context("write policy.json")?;
    Ok(())
}

/// Fetch fresh policy from the server. Returns an error on any failure so
/// the caller can log it without overwriting the cached policy. The caller
/// is responsible for persisting on success.
pub async fn fetch_once(server: &str, token: &str) -> Result<Policy> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let resp = client
        .get(format!("{}/api/v1/config", server.trim_end_matches('/')))
        .bearer_auth(token)
        .send()
        .await
        .context("GET /api/v1/config")?;
    if !resp.status().is_success() {
        return Err(anyhow!("config HTTP {}", resp.status()));
    }
    let j: serde_json::Value = resp.json().await?;

    let p = j.get("policy").ok_or_else(|| anyhow!("missing policy"))?;
    let get_u64 = |k: &str, dflt: u64| p.get(k).and_then(|v| v.as_u64()).unwrap_or(dflt);
    let get_bool = |k: &str, dflt: bool| p.get(k).and_then(|v| v.as_bool()).unwrap_or(dflt);
    let version = j
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(Policy {
        capture_screenshots: get_bool("capture_screenshots", true),
        screenshot_interval_secs: get_u64("screenshot_interval_secs", 180),
        activity_sample_interval_secs: get_u64("activity_sample_interval_secs", 30),
        app_snapshot_interval_secs: get_u64("app_snapshot_interval_secs", 60),
        idle_threshold_secs: get_u64("idle_threshold_secs", 120),
        screenshot_quality: get_u64("screenshot_quality", 50) as u32,
        screenshot_max_width: get_u64("screenshot_max_width", 1280) as u32,
        refresh_interval_secs: get_u64("refresh_interval_secs", 300),
        version,
        from_server: true,
    })
}

/// Try a single refresh: fetch, persist, swap into shared state. Logs on
/// failure but doesn't return an error — policy fetch must never crash the
/// host loop.
pub async fn try_refresh(state: &AppState) {
    if state.auth.read().user.is_none() {
        return;
    }
    let token = match crate::auth::ensure_fresh_token(state).await {
        Ok(t) => t,
        Err(e) => {
            log::debug!("policy: no token yet: {e:?}");
            return;
        }
    };
    let server = state.server_url();
    match fetch_once(&server, &token).await {
        Ok(p) => {
            if let Err(e) = save_to_disk(state, &p).await {
                log::warn!("policy: persist failed: {e:?}");
            }
            *state.policy.write() = p;
            log::info!("policy: refreshed from server");
        }
        Err(e) => log::warn!("policy: refresh failed: {e:?}"),
    }
}

/// Background loop: pull policy at startup then re-pull every
/// `refresh_interval_secs`. Honors the latest value the server returns so
/// admins can speed up or slow down rollouts by changing that field.
pub async fn start_refresh_loop(state: AppState) {
    tokio::spawn(async move {
        // Small initial delay so we don't race the first sync_loop tick
        // and trigger two token refreshes back-to-back.
        sleep(Duration::from_secs(5)).await;
        loop {
            try_refresh(&state).await;
            let next = state.policy.read().refresh_interval_secs.max(60);
            sleep(Duration::from_secs(next)).await;
        }
    });
}
