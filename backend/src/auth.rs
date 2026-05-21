use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use base64::Engine;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::models::User;
use crate::state::AppState;

pub const SESSION_USER_KEY: &str = "uid";

/// Lifetime of an access token (JWT). Kept short so revocation latency is
/// bounded by this value — the desktop app refreshes on demand using its
/// refresh token, which is the actual revocation point.
pub const ACCESS_TOKEN_TTL_SECS: i64 = 3600;

/// Lifetime of a refresh token. Long enough to survive vacations but short
/// enough that a leaked refresh token eventually stops working without
/// admin intervention.
pub const REFRESH_TOKEN_TTL_SECS: i64 = 60 * 60 * 24 * 60; // 60 days

#[derive(Clone, Debug)]
pub struct CurrentUser(pub User);

#[axum::async_trait]
impl FromRequestParts<Arc<AppState>> for CurrentUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let session = tower_sessions::Session::from_request_parts(parts, state)
            .await
            .map_err(|_| Redirect::to("/admin/login").into_response())?;

        let uid: Option<String> = session.get(SESSION_USER_KEY).await.ok().flatten();
        let Some(uid) = uid else {
            return Err(Redirect::to("/admin/login").into_response());
        };

        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(&uid)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| AppError::Internal(e.to_string()).into_response())?;

        match user {
            Some(u) => Ok(CurrentUser(u)),
            None => {
                let _ = session.delete().await;
                Err(Redirect::to("/admin/login").into_response())
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct AdminUser(pub User);

#[axum::async_trait]
impl FromRequestParts<Arc<AppState>> for AdminUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let CurrentUser(u) = CurrentUser::from_request_parts(parts, state).await?;
        if !u.is_admin_bool() {
            return Err((StatusCode::FORBIDDEN, "Admin only").into_response());
        }
        Ok(AdminUser(u))
    }
}

/// Extractor for an authenticated user who is at least a member (admins
/// included). Non-members are redirected to /pending.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MemberUser(pub User);

#[allow(dead_code)]
#[axum::async_trait]
impl FromRequestParts<Arc<AppState>> for MemberUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let CurrentUser(u) = CurrentUser::from_request_parts(parts, state).await?;
        if !u.is_member_bool() {
            return Err(Redirect::to("/pending").into_response());
        }
        Ok(MemberUser(u))
    }
}

// ---------- JWT access tokens ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: String,  // user id
    pub did: String,  // device id
    pub tier: String, // "admin" | "member"
    pub iat: i64,
    pub exp: i64,
    /// Unique per token. Without it, two tokens minted in the same second
    /// for the same (user, device, tier) hash to the same JWT string — the
    /// refresh endpoint then can't return a token that looks different
    /// from the previous one even though server state has rotated.
    pub jti: String,
}

pub fn encode_access_token(
    state: &AppState,
    user_id: &str,
    device_id: &str,
    tier: &str,
) -> Result<(String, i64), AppError> {
    let now = chrono::Utc::now().timestamp();
    let exp = now + ACCESS_TOKEN_TTL_SECS;
    let claims = AccessClaims {
        sub: user_id.to_string(),
        did: device_id.to_string(),
        tier: tier.to_string(),
        iat: now,
        exp,
        jti: uuid::Uuid::new_v4().to_string(),
    };
    let key = EncodingKey::from_secret(state.config.jwt_secret.as_bytes());
    let token = jsonwebtoken::encode(&Header::default(), &claims, &key)
        .map_err(|e| AppError::Internal(format!("jwt encode: {e}")))?;
    Ok((token, exp))
}

pub fn decode_access_token(state: &AppState, token: &str) -> Result<AccessClaims, AppError> {
    let key = DecodingKey::from_secret(state.config.jwt_secret.as_bytes());
    let mut validation = Validation::default();
    validation.leeway = 5;
    let data = jsonwebtoken::decode::<AccessClaims>(token, &key, &validation)
        .map_err(|_| AppError::Unauthorized)?;
    Ok(data.claims)
}

/// Extractor for device-authenticated requests from the desktop app. The
/// bearer credential is a JWT access token issued by `complete_desktop_flow`.
#[derive(Clone, Debug)]
pub struct DeviceAuth {
    pub user: User,
    pub device_id: String,
}

#[axum::async_trait]
impl FromRequestParts<Arc<AppState>> for DeviceAuth {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|v| v.to_string());

        let Some(token) = token else {
            return Err((StatusCode::UNAUTHORIZED, "missing bearer token").into_response());
        };

        let claims = decode_access_token(state, &token)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid token").into_response())?;

        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(&claims.sub)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| AppError::Internal(e.to_string()).into_response())?;

        let Some(user) = user else {
            return Err((StatusCode::UNAUTHORIZED, "user gone").into_response());
        };

        // Membership can be revoked by an admin after the token was issued —
        // re-check on every request so a revocation takes effect immediately.
        if !user.is_member_bool() {
            return Err((StatusCode::FORBIDDEN, "membership revoked").into_response());
        }

        // The device row must still exist and not be revoked. This is the
        // single source of truth for "is this device allowed?".
        let device: Option<(String, Option<i64>)> =
            sqlx::query_as("SELECT id, revoked_at FROM devices WHERE id = ? AND user_id = ?")
                .bind(&claims.did)
                .bind(&user.id)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| AppError::Internal(e.to_string()).into_response())?;

        let Some((device_id, revoked_at)) = device else {
            return Err((StatusCode::UNAUTHORIZED, "device unknown").into_response());
        };
        if revoked_at.is_some() {
            return Err((StatusCode::UNAUTHORIZED, "device revoked").into_response());
        }

        let now = chrono::Utc::now().timestamp();
        let _ = sqlx::query("UPDATE devices SET last_seen_at = ? WHERE id = ?")
            .bind(now)
            .bind(&device_id)
            .execute(&state.db)
            .await;

        Ok(DeviceAuth { user, device_id })
    }
}

pub fn hash_token(t: &str) -> String {
    let mut h = Sha256::new();
    h.update(t.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(h.finalize())
}

pub fn random_token() -> String {
    use rand::RngCore;
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
}

pub fn tier_for(user: &User) -> &'static str {
    if user.is_admin_bool() {
        "admin"
    } else {
        "member"
    }
}

pub async fn ensure_user_from_google(
    state: &AppState,
    google_sub: &str,
    email: &str,
    name: Option<&str>,
    picture: Option<&str>,
) -> Result<User, AppError> {
    let lc_email = email.to_lowercase();
    let is_admin = if state.config.is_admin_email(&lc_email) {
        1
    } else {
        0
    };
    let is_member = compute_is_member(state, &lc_email).await? as i64;

    // Try existing user
    if let Some(u) =
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE google_sub = ? OR email = ?")
            .bind(google_sub)
            .bind(&lc_email)
            .fetch_optional(&state.db)
            .await?
    {
        sqlx::query(
            "UPDATE users SET email = ?, name = ?, picture = ?, google_sub = ?, is_admin = ?, is_member = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&lc_email)
        .bind(name)
        .bind(picture)
        .bind(google_sub)
        .bind(is_admin)
        .bind(is_member)
        .bind(&u.id)
        .execute(&state.db)
        .await?;
        let u = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(&u.id)
            .fetch_one(&state.db)
            .await?;
        return Ok(u);
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO users (id, email, name, picture, is_admin, is_member, google_sub) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&lc_email)
    .bind(name)
    .bind(picture)
    .bind(is_admin)
    .bind(is_member)
    .bind(google_sub)
    .execute(&state.db)
    .await?;

    let u = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;
    Ok(u)
}

/// Membership check combines: env list ∪ allowed_members table. Admins are
/// always considered members regardless.
pub async fn compute_is_member(state: &AppState, lc_email: &str) -> Result<bool, AppError> {
    if state.config.is_admin_email(lc_email) {
        return Ok(true);
    }
    if state.config.is_member_email_env(lc_email) {
        return Ok(true);
    }
    let exists: Option<String> =
        sqlx::query_scalar("SELECT email FROM allowed_members WHERE email = ?")
            .bind(lc_email)
            .fetch_optional(&state.db)
            .await?;
    Ok(exists.is_some())
}

/// Recompute and persist `is_member` for the given user — used after admin
/// edits the allow-list so the existing session reflects new status on next
/// page load.
pub async fn refresh_user_membership(state: &AppState, user_id: &str) -> Result<(), AppError> {
    let email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?;
    let Some(email) = email else { return Ok(()) };
    let is_member = compute_is_member(state, &email).await? as i64;
    sqlx::query("UPDATE users SET is_member = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(is_member)
        .bind(user_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

// Convenience helper: extract optional current user without redirecting.
pub async fn current_user_opt(
    state: &Arc<AppState>,
    session: &tower_sessions::Session,
) -> Option<User> {
    let uid: Option<String> = session.get(SESSION_USER_KEY).await.ok().flatten()?;
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&uid)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
}
