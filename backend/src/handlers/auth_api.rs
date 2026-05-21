//! Token lifecycle endpoints used by the desktop app after the initial
//! browser login flow has completed.
//!
//! - POST /api/v1/auth/refresh — rotate refresh token, mint new access token.
//! - GET  /api/v1/auth/verify  — sanity-check a Bearer access token.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{
    encode_access_token, hash_token, random_token, tier_for, DeviceAuth, REFRESH_TOKEN_TTL_SECS,
};
use crate::error::{AppError, AppResult};
use crate::models::User;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RefreshReq {
    pub refresh_token: String,
    pub device_fingerprint: String,
}

#[derive(Serialize)]
pub struct RefreshResp {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub subscription: Subscription,
}

#[derive(Serialize)]
pub struct Subscription {
    pub tier: String,
}

/// Hand a desktop client a fresh access token and rotate its refresh
/// token. Rejecting this call (HTTP 401) is the signal for the desktop
/// app to clear its keyring entry — every other error keeps the token
/// usable so a flaky network doesn't log the user out.
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RefreshReq>,
) -> Result<Json<RefreshResp>, Response> {
    let hash = hash_token(&body.refresh_token);
    let now = chrono::Utc::now().timestamp();

    // Look up the active session row by token hash. A revoked or rotated
    // row is treated as "token rejected" so the client clears its state.
    // (id, user_id, device_id, expires_at, rotated_at, revoked_at)
    type SessionRow = (String, String, String, i64, Option<i64>, Option<i64>);
    let row: Option<SessionRow> = sqlx::query_as(
        "SELECT id, user_id, device_id, expires_at, rotated_at, revoked_at
           FROM auth_sessions WHERE refresh_token_hash = ?",
    )
    .bind(&hash)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(e.to_string()).into_response())?;

    let Some((session_id, user_id, device_id, expires_at, rotated_at, revoked_at)) = row else {
        return Err(unauthorized("token_rejected"));
    };
    if rotated_at.is_some() || revoked_at.is_some() || now > expires_at {
        return Err(unauthorized("token_rejected"));
    }

    // Device fingerprint binding: even with a valid token, a different
    // machine can't use it.
    let dev: Option<(String, Option<i64>)> =
        sqlx::query_as("SELECT fingerprint, revoked_at FROM devices WHERE id = ? AND user_id = ?")
            .bind(&device_id)
            .bind(&user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| AppError::Internal(e.to_string()).into_response())?;

    let Some((dev_fp, dev_revoked)) = dev else {
        return Err(unauthorized("device_unknown"));
    };
    if dev_revoked.is_some() {
        return Err(unauthorized("device_revoked"));
    }
    if dev_fp != body.device_fingerprint {
        return Err(unauthorized("fingerprint_mismatch"));
    }

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()).into_response())?
        .ok_or_else(|| unauthorized("user_gone"))?;
    if !user.is_member_bool() {
        return Err(unauthorized("membership_revoked"));
    }

    // Rotate: mark old row rotated, insert a fresh one. Strict rotation
    // (old token instantly invalid) is the standard mitigation for token
    // replay if the previous one leaks.
    let new_token = random_token();
    let new_hash = hash_token(&new_token);
    let new_session_id = uuid::Uuid::new_v4().to_string();

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| AppError::Internal(e.to_string()).into_response())?;
    sqlx::query("UPDATE auth_sessions SET rotated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&session_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(e.to_string()).into_response())?;
    sqlx::query(
        "INSERT INTO auth_sessions
           (id, user_id, device_id, refresh_token_hash, issued_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&new_session_id)
    .bind(&user.id)
    .bind(&device_id)
    .bind(&new_hash)
    .bind(now)
    .bind(now + REFRESH_TOKEN_TTL_SECS)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(e.to_string()).into_response())?;
    tx.commit()
        .await
        .map_err(|e| AppError::Internal(e.to_string()).into_response())?;

    let tier = tier_for(&user);
    let (access_token, access_exp) =
        encode_access_token(&state, &user.id, &device_id, tier).map_err(|e| e.into_response())?;

    let _ = sqlx::query("UPDATE devices SET last_seen_at = ? WHERE id = ?")
        .bind(now)
        .bind(&device_id)
        .execute(&state.db)
        .await;

    Ok(Json(RefreshResp {
        access_token,
        refresh_token: new_token,
        expires_at: access_exp,
        subscription: Subscription {
            tier: tier.to_string(),
        },
    }))
}

fn unauthorized(reason: &'static str) -> Response {
    (StatusCode::UNAUTHORIZED, Json(json!({ "error": reason }))).into_response()
}

#[derive(Serialize)]
pub struct VerifyResp {
    pub valid: bool,
    pub user: VerifyUser,
    pub subscription: Subscription,
}

#[derive(Serialize)]
pub struct VerifyUser {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub is_admin: bool,
    pub is_member: bool,
}

pub async fn verify(auth: DeviceAuth) -> AppResult<Json<VerifyResp>> {
    let tier = tier_for(&auth.user);
    Ok(Json(VerifyResp {
        valid: true,
        user: VerifyUser {
            id: auth.user.id.clone(),
            email: auth.user.email.clone(),
            name: auth.user.name.clone(),
            picture: auth.user.picture.clone(),
            is_admin: auth.user.is_admin_bool(),
            is_member: auth.user.is_member_bool(),
        },
        subscription: Subscription {
            tier: tier.to_string(),
        },
    }))
}
