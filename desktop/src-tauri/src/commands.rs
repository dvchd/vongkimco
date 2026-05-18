use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::db::LocalSession;
use crate::settings::Settings;
use crate::state::{
    emit_status, save_auth_to_disk, save_settings_to_disk, AppState, PendingLinkRequest, UserInfo,
};

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

#[derive(Serialize, Deserialize)]
pub struct DeviceLinkStartResp {
    pub user_code: String,
    pub verification_url: String,
}

#[tauri::command]
pub async fn device_link_start(state: State<'_, AppState>) -> CmdResult<DeviceLinkStartResp> {
    let server = state.server_url();
    let hostname = hostname_or_default();
    let platform = std::env::consts::OS.to_string();

    let body = serde_json::json!({ "device_name": hostname, "platform": platform });
    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "{}/api/v1/device/link/start",
            server.trim_end_matches('/')
        ))
        .json(&body)
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let j: Value = resp.json().await.map_err(err)?;
    let device_code = j["device_code"].as_str().unwrap_or("").to_string();
    let user_code = j["user_code"].as_str().unwrap_or("").to_string();
    let verification_url = j["verification_url"].as_str().unwrap_or("").to_string();

    *state.pending_link.write() = Some(PendingLinkRequest {
        device_code: device_code.clone(),
        user_code: user_code.clone(),
        verification_url: verification_url.clone(),
    });

    Ok(DeviceLinkStartResp {
        user_code,
        verification_url,
    })
}

#[tauri::command]
pub async fn device_link_poll(state: State<'_, AppState>) -> CmdResult<Value> {
    let server = state.server_url();
    let pending = state.pending_link.read().clone();
    let Some(p) = pending else {
        return Err("no pending link".into());
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "{}/api/v1/device/link/poll",
            server.trim_end_matches('/')
        ))
        .json(&serde_json::json!({ "device_code": p.device_code }))
        .send()
        .await
        .map_err(err)?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let j: Value = resp.json().await.map_err(err)?;
    let status = j["status"].as_str().unwrap_or("pending").to_string();

    if status == "approved" {
        let token = j["token"].as_str().unwrap_or("").to_string();
        let user = j.get("user").cloned().unwrap_or(Value::Null);
        let user_info: Option<UserInfo> = serde_json::from_value(user).ok();
        {
            let mut a = state.auth.write();
            a.token = Some(token);
            a.user = user_info;
        }
        save_auth_to_disk(&state).await.map_err(err)?;
        *state.pending_link.write() = None;
    } else if status == "expired" {
        *state.pending_link.write() = None;
    }

    Ok(j)
}

#[tauri::command]
pub async fn get_current_user(state: State<'_, AppState>) -> CmdResult<Option<UserInfo>> {
    Ok(state.auth.read().user.clone())
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> CmdResult<()> {
    // Stop any running session
    let _ = do_stop_session(&state).await;
    *state.auth.write() = Default::default();
    save_auth_to_disk(&state).await.map_err(err)?;
    Ok(())
}

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
    let token = state.auth_token().ok_or("not logged in".to_string())?;
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

fn hostname_or_default() -> String {
    if let Ok(h) = std::env::var("COMPUTERNAME") {
        return h;
    }
    if let Ok(h) = std::env::var("HOSTNAME") {
        return h;
    }
    "desktop".to_string()
}
