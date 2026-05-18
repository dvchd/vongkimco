//! Google OAuth callback shared by the admin web UI and the desktop login
//! flow. The two flows are differentiated by `state`:
//!
//! - If `state` matches a pending row in `login_flows` → desktop flow:
//!   complete it, redirect the browser to `/auth/desktop/done`.
//! - Otherwise → web admin flow: require `session.oauth_state` match,
//!   set the session cookie, redirect to a role-aware landing page.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use tower_sessions::Session;

use crate::auth::{ensure_user_from_google, SESSION_USER_KEY};
use crate::error::{AppError, AppResult};
use crate::handlers::desktop_auth::complete_desktop_flow_if_match;
use crate::state::AppState;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

fn admin_redirect_uri(state: &AppState) -> String {
    format!(
        "{}/admin/oauth/callback",
        state.config.public_url.trim_end_matches('/')
    )
}

pub async fn admin_login_start(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> AppResult<Response> {
    let state_token = crate::auth::random_token();
    session
        .insert("oauth_state", state_token.clone())
        .await
        .map_err(|e| AppError::Internal(format!("session: {e}")))?;

    let redirect_uri = admin_redirect_uri(&state);
    let url = format!(
        "{base}?client_id={cid}&redirect_uri={ru}&response_type=code&scope=openid%20email%20profile&state={st}&access_type=online&prompt=select_account",
        base = GOOGLE_AUTH_URL,
        cid = urlencoding::encode(&state.config.google_client_id),
        ru = urlencoding::encode(&redirect_uri),
        st = urlencoding::encode(&state_token),
    );
    Ok(Redirect::to(&url).into_response())
}

#[derive(Deserialize)]
struct TokenResp {
    access_token: String,
    #[serde(default)]
    id_token: Option<String>,
}

#[derive(Deserialize)]
struct UserInfo {
    sub: String,
    email: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    picture: Option<String>,
    #[serde(default)]
    email_verified: Option<bool>,
}

pub async fn admin_oauth_callback(
    State(state): State<Arc<AppState>>,
    session: Session,
    Query(q): Query<CallbackQuery>,
) -> AppResult<Response> {
    if let Some(err) = q.error {
        return Err(AppError::BadRequest(format!("oauth: {err}")));
    }
    let Some(code) = q.code else {
        return Err(AppError::BadRequest("missing code".into()));
    };
    let Some(qstate) = q.state.clone() else {
        return Err(AppError::BadRequest("missing state".into()));
    };

    // Is this a desktop-flow callback? Check before we touch the session
    // cookie state, because desktop flows are stateless from the browser's
    // perspective (the user might be in a fresh incognito window).
    let desktop_flow_match: bool = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM login_flows WHERE state = ? AND status = 'pending'",
    )
    .bind(&qstate)
    .fetch_one(&state.db)
    .await?
        > 0;

    if !desktop_flow_match {
        // Web admin flow — enforce the session-tied CSRF token.
        let saved_state: Option<String> = session.get("oauth_state").await.ok().flatten();
        if saved_state.as_deref() != Some(qstate.as_str()) {
            return Err(AppError::BadRequest("state mismatch".into()));
        }
        let _ = session.remove::<String>("oauth_state").await;
    }

    // Exchange code → user info. Same for both flows.
    let client = reqwest::Client::new();
    let redirect_uri = admin_redirect_uri(&state);
    let resp = client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("code", code.as_str()),
            ("client_id", state.config.google_client_id.as_str()),
            ("client_secret", state.config.google_client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await?
        .error_for_status()?;
    let tok: TokenResp = resp.json().await?;

    let info: UserInfo = client
        .get(GOOGLE_USERINFO_URL)
        .bearer_auth(&tok.access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    if matches!(info.email_verified, Some(false)) {
        return Err(AppError::Forbidden);
    }

    let user = ensure_user_from_google(
        &state,
        &info.sub,
        &info.email,
        info.name.as_deref(),
        info.picture.as_deref(),
    )
    .await?;

    let _ = tok.id_token; // not needed; we used the userinfo endpoint

    if desktop_flow_match {
        let outcome = complete_desktop_flow_if_match(&state, &qstate, &user).await?;
        if outcome.matched {
            return Ok(Redirect::to("/auth/desktop/done").into_response());
        }
        // Race: the flow expired between the COUNT and the completion. Fall
        // through to the web-flow branch so the user isn't stranded.
    }

    session
        .insert(SESSION_USER_KEY, user.id.clone())
        .await
        .map_err(|e| AppError::Internal(format!("session: {e}")))?;

    let dest = if user.is_admin_bool() {
        "/admin"
    } else if user.is_member_bool() {
        "/feedback"
    } else {
        "/pending"
    };
    Ok(Redirect::to(dest).into_response())
}

pub async fn admin_logout(session: Session) -> Response {
    let _ = session.delete().await;
    Redirect::to("/admin/login").into_response()
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}
