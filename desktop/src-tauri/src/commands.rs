use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::Value;
use tauri::State;

use crate::auth::{self, PollOutcome, StartLoginResp};
use crate::db::LocalSession;
use crate::settings::Settings;
use crate::state::{emit_status, save_settings_to_disk, AppState, UserInfo};

type CmdResult<T> = std::result::Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> CmdResult<Settings> {
    Ok(state.settings.read().clone())
}

#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: Settings) -> CmdResult<()> {
    *state.settings.write() = settings;
    save_settings_to_disk(&state).await.map_err(err)?;
    // Re-register hotkeys with new bindings
    crate::monitor::register_hotkeys((*state).clone()).await;

    // Apply autostart preference
    apply_autostart(&state).await.map_err(err)?;
    Ok(())
}

async fn apply_autostart(state: &AppState) -> Result<()> {
    use tauri_plugin_autostart::ManagerExt;
    let want = state.settings.read().autostart;
    let manager = state.app.autolaunch();
    let is_enabled = manager.is_enabled().unwrap_or(false);
    if want && !is_enabled {
        let _ = manager.enable();
    } else if !want && is_enabled {
        let _ = manager.disable();
    }
    Ok(())
}

#[tauri::command]
pub async fn set_server_url(state: State<'_, AppState>, url: String) -> CmdResult<()> {
    {
        let mut s = state.settings.write();
        s.server_url = url;
    }
    save_settings_to_disk(&state).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn test_server(url: String) -> CmdResult<Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(err)?;
    let resp = client
        .get(format!("{}/api/v1/server-info", url.trim_end_matches('/')))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let j: Value = resp.json().await.map_err(err)?;
    Ok(j)
}

// ---------- Auth commands ----------

#[tauri::command]
pub async fn auth_start(state: State<'_, AppState>) -> CmdResult<StartLoginResp> {
    auth::start_login(&state).await.map_err(err)
}

/// Poll the server for the desktop flow's status. Returns one of:
///   { status: "pending" }
///   { status: "expired" }
///   { status: "device_limit_exceeded" }
///   { status: "not_member" }
///   { status: "completed", user: {...} }   // tokens are stashed server-side
#[tauri::command]
pub async fn auth_poll(state: State<'_, AppState>) -> CmdResult<Value> {
    let outcome = auth::poll_login(&state).await.map_err(err)?;
    let v = match outcome {
        PollOutcome::Pending => serde_json::json!({ "status": "pending" }),
        PollOutcome::Expired => serde_json::json!({ "status": "expired" }),
        PollOutcome::DeviceLimitExceeded => {
            serde_json::json!({ "status": "device_limit_exceeded" })
        }
        PollOutcome::NotMember => serde_json::json!({ "status": "not_member" }),
        PollOutcome::Completed { user, .. } => serde_json::json!({
            "status": "completed",
            "user": user,
        }),
    };
    Ok(v)
}

#[tauri::command]
pub async fn auth_cancel(state: State<'_, AppState>) -> CmdResult<()> {
    auth::cancel_login(&state);
    Ok(())
}

#[tauri::command]
pub async fn get_current_user(state: State<'_, AppState>) -> CmdResult<Option<UserInfo>> {
    Ok(state.auth.read().user.clone())
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> CmdResult<()> {
    // Stop any running session
    let _ = do_stop_session(&state).await;
    auth::logout(&state).await.map_err(err)?;
    Ok(())
}

// ---------- Session commands ----------

#[tauri::command]
pub async fn start_session(state: State<'_, AppState>, note: Option<String>) -> CmdResult<String> {
    do_start_session(&state, note).await.map_err(err)
}

pub async fn do_start_session(state: &AppState, note: Option<String>) -> Result<String> {
    {
        let s = state.session.read();
        if s.running {
            return Err(anyhow!("session already running"));
        }
    }
    let started_at = chrono::Utc::now().to_rfc3339();
    let id = state.db.create_session(&started_at, note.as_deref())?;
    {
        let mut s = state.session.write();
        s.running = true;
        s.session_id = Some(id.clone());
        s.started_at = Some(started_at);
        s.keyboard_events = 0;
        s.mouse_events = 0;
        s.screenshots_taken = 0;
        s.last_activity = "active".into();
    }
    emit_status(state);

    // Best-effort notification
    use tauri_plugin_notification::NotificationExt;
    let _ = state
        .app
        .notification()
        .builder()
        .title("Vòng Kim Cô")
        .body("Phiên làm việc đã bắt đầu")
        .show();

    Ok(id)
}

#[tauri::command]
pub async fn stop_session(state: State<'_, AppState>) -> CmdResult<()> {
    do_stop_session(&state).await.map_err(err)
}

pub async fn do_stop_session(state: &AppState) -> Result<()> {
    let id = state.session.read().session_id.clone();
    let Some(id) = id else { return Ok(()) };
    let ended_at = chrono::Utc::now().to_rfc3339();
    state.db.end_session(&id, &ended_at)?;
    {
        let mut s = state.session.write();
        s.running = false;
        s.session_id = None;
        s.started_at = None;
    }
    emit_status(state);

    use tauri_plugin_notification::NotificationExt;
    let _ = state
        .app
        .notification()
        .builder()
        .title("Vòng Kim Cô")
        .body("Phiên làm việc đã kết thúc")
        .show();
    Ok(())
}

#[tauri::command]
pub async fn sync_now(state: State<'_, AppState>) -> CmdResult<()> {
    let token = auth::ensure_fresh_token(&state).await.map_err(err)?;
    let server = state.server_url();
    crate::sync::sync_once(&state, &server, &token)
        .await
        .map_err(err)?;
    crate::sync::update_pending_counts(&state);
    emit_status(&state);
    Ok(())
}

#[tauri::command]
pub async fn get_session_state(
    state: State<'_, AppState>,
) -> CmdResult<crate::state::SessionState> {
    Ok(state.session.read().clone())
}

#[tauri::command]
pub async fn list_local_sessions(state: State<'_, AppState>) -> CmdResult<Vec<LocalSession>> {
    state.db.list_sessions(200).map_err(err)
}

/// Exposed so the UI can show the fingerprint on a "Devices" / debug page
/// if we ever want to. Useful for sanity-checking machine binding.
#[tauri::command]
pub async fn get_device_fingerprint() -> CmdResult<String> {
    Ok(auth::device_fingerprint())
}

#[derive(Serialize)]
pub struct AuthStatus {
    pub user: Option<UserInfo>,
    pub has_refresh_token: bool,
}

#[tauri::command]
pub async fn get_auth_status(state: State<'_, AppState>) -> CmdResult<AuthStatus> {
    Ok(AuthStatus {
        user: state.auth.read().user.clone(),
        has_refresh_token: auth::load_refresh_token().is_some(),
    })
}
