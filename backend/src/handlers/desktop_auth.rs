//! Desktop "device link" flow.
//!
//! 1. Desktop POSTs /api/v1/device/link/start  → { device_code, user_code, verification_url }
//! 2. Desktop opens the verification_url in the user's browser
//! 3. User signs in with Google (admin OAuth) and approves the device
//! 4. Desktop polls POST /api/v1/device/link/poll with { device_code }
//!    → { status: "pending"|"approved", token?: "..." }

use std::sync::Arc;

use axum::extract::{Form, Path, Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_sessions::Session;

use crate::auth::{current_user_opt, hash_token, random_token};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

const LINK_TTL_SECONDS: i64 = 600;

#[derive(Deserialize)]
pub struct StartReq {
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
}

#[derive(Serialize)]
pub struct StartResp {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_in: i64,
    pub interval: i64,
}

fn generate_user_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // no confusing chars
    let mut rng = rand::thread_rng();
    let raw: String = (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..ALPHABET.len());
            ALPHABET[idx] as char
        })
        .collect();
    format!("{}-{}", &raw[..4], &raw[4..])
}

pub async fn device_link_start(
    State(state): State<Arc<AppState>>,
    Json(body): Json<StartReq>,
) -> AppResult<Json<StartResp>> {
    let device_code = random_token();
    let user_code = generate_user_code();
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(LINK_TTL_SECONDS);

    sqlx::query(
        "INSERT INTO device_links (device_code, user_code, device_name, platform, expires_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&device_code)
    .bind(&user_code)
    .bind(&body.device_name)
    .bind(&body.platform)
    .bind(expires_at.to_rfc3339())
    .execute(&state.db)
    .await?;

    let verification_url = format!(
        "{}/device/activate?code={}",
        state.config.public_url.trim_end_matches('/'),
        url::form_urlencoded::byte_serialize(user_code.as_bytes()).collect::<String>()
    );

    Ok(Json(StartResp {
        device_code,
        user_code,
        verification_url,
        expires_in: LINK_TTL_SECONDS,
        interval: 3,
    }))
}

#[derive(Deserialize)]
pub struct PollReq {
    pub device_code: String,
}

pub async fn device_link_poll(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PollReq>,
) -> AppResult<Json<Value>> {
    let row = sqlx::query_as::<
        _,
        (
            String,
            Option<String>,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT user_code, user_id, issued_token_id, expires_at, device_name, platform
         FROM device_links WHERE device_code = ?",
    )
    .bind(&body.device_code)
    .fetch_optional(&state.db)
    .await?;

    let Some((_user_code, user_id, issued_token_id, expires_at, device_name, platform)) = row
    else {
        return Err(AppError::NotFound);
    };

    let exp = chrono::DateTime::parse_from_rfc3339(&expires_at)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    if exp < chrono::Utc::now() {
        return Ok(Json(json!({ "status": "expired" })));
    }

    let Some(uid) = user_id else {
        return Ok(Json(json!({ "status": "pending" })));
    };

    if let Some(tid) = issued_token_id {
        // token already issued (rare race); refuse to re-issue
        return Ok(Json(json!({ "status": "already_issued", "token_id": tid })));
    }

    // Issue token
    let raw_token = random_token();
    let token_hash = hash_token(&raw_token);
    let token_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO device_tokens (id, user_id, token_hash, device_name, platform) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&token_id)
    .bind(&uid)
    .bind(&token_hash)
    .bind(&device_name)
    .bind(&platform)
    .execute(&state.db)
    .await?;
    sqlx::query("UPDATE device_links SET issued_token_id = ? WHERE device_code = ?")
        .bind(&token_id)
        .bind(&body.device_code)
        .execute(&state.db)
        .await?;

    // Fetch user info for the response
    let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = ?")
        .bind(&uid)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(json!({
        "status": "approved",
        "token": raw_token,
        "token_id": token_id,
        "user": {
            "id": user.id,
            "email": user.email,
            "name": user.name,
            "picture": user.picture,
        }
    })))
}

#[derive(Deserialize)]
pub struct ActivateQuery {
    code: Option<String>,
}

/// GET /device/activate?code=XXXX-XXXX — public page that asks user to log in
/// and then approves the device link. Renders with Askama from admin templates.
pub async fn device_activate_page(
    State(state): State<Arc<AppState>>,
    session: Session,
    Query(q): Query<ActivateQuery>,
) -> AppResult<Response> {
    let user = current_user_opt(&state, &session).await;
    let code = q.code.clone().unwrap_or_default();

    if user.is_none() {
        // Stash code in session so we can return after login
        let _ = session.insert("pending_device_code", code.clone()).await;
        return Ok(Redirect::to("/admin/login?reason=device").into_response());
    }

    let user = user.unwrap();

    let tmpl = crate::handlers::admin::ActivateTemplate {
        code,
        user_email: user.email.clone(),
        error: None,
    };
    Ok(tmpl.into_response())
}

#[derive(Deserialize)]
pub struct ActivateForm {
    pub code: String,
}

/// POST /device/activate — approves the device link for the current user.
pub async fn device_activate_submit(
    State(state): State<Arc<AppState>>,
    session: Session,
    Form(form): Form<ActivateForm>,
) -> AppResult<Response> {
    let Some(user) = current_user_opt(&state, &session).await else {
        return Ok(Redirect::to("/admin/login?reason=device").into_response());
    };

    // Only members (and admins) can pair a device with this server.
    if !user.is_member_bool() {
        let tmpl = crate::handlers::admin::ActivateTemplate {
            code: form.code,
            user_email: user.email,
            error: Some(
                "Tài khoản của bạn chưa được duyệt làm thành viên. Vui lòng liên hệ quản trị viên."
                    .into(),
            ),
        };
        return Ok(tmpl.into_response());
    }

    let code = form.code.trim().to_uppercase();
    let result = sqlx::query(
        "UPDATE device_links SET user_id = ?, approved_at = datetime('now')
         WHERE user_code = ? AND user_id IS NULL AND expires_at > datetime('now')",
    )
    .bind(&user.id)
    .bind(&code)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        let tmpl = crate::handlers::admin::ActivateTemplate {
            code,
            user_email: user.email,
            error: Some("Mã không hợp lệ hoặc đã hết hạn".into()),
        };
        return Ok(tmpl.into_response());
    }

    let tmpl = crate::handlers::admin::ActivateDoneTemplate {
        user_email: user.email,
    };
    Ok(tmpl.into_response())
}

pub async fn _device_revoke(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    sqlx::query("UPDATE device_tokens SET revoked_at = datetime('now') WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}
