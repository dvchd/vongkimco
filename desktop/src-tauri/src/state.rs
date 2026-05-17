use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::db::LocalDb;
use crate::settings::Settings;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SessionState {
    pub running: bool,
    pub session_id: Option<String>,
    pub started_at: Option<String>,
    pub last_activity: String, // "active" | "idle"
    pub keyboard_events: i64,
    pub mouse_events: i64,
    pub screenshots_taken: i64,
    pub pending_sync: i64,
    pub online: bool,
}

impl SessionState {
    fn initial() -> Self {
        Self {
            running: false,
            session_id: None,
            started_at: None,
            last_activity: "active".into(),
            keyboard_events: 0,
            mouse_events: 0,
            screenshots_taken: 0,
            pending_sync: 0,
            online: false,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PendingLinkRequest {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
}

/// In-memory cross-cutting application state. Wrap in Arc<...> for cheap cloning.
#[derive(Clone)]
pub struct AppState {
    pub app: AppHandle,
    pub data_dir: PathBuf,
    pub screenshot_dir: PathBuf,
    pub db: Arc<LocalDb>,
    pub settings: Arc<RwLock<Settings>>,
    pub auth: Arc<RwLock<AuthStore>>,
    pub session: Arc<RwLock<SessionState>>,
    pub pending_link: Arc<RwLock<Option<PendingLinkRequest>>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AuthStore {
    pub token: Option<String>,
    pub user: Option<UserInfo>,
}

impl AppState {
    pub fn new(app: AppHandle) -> Self {
        let data_dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("./vkc-data"));
        std::fs::create_dir_all(&data_dir).ok();
        let screenshot_dir = data_dir.join("screenshots");
        std::fs::create_dir_all(&screenshot_dir).ok();

        let db = LocalDb::open(&data_dir).expect("init local db");

        Self {
            app,
            data_dir,
            screenshot_dir,
            db: Arc::new(db),
            settings: Arc::new(RwLock::new(Settings::default())),
            auth: Arc::new(RwLock::new(AuthStore::default())),
            session: Arc::new(RwLock::new(SessionState::initial())),
            pending_link: Arc::new(RwLock::new(None)),
        }
    }

    pub fn auth_token(&self) -> Option<String> {
        self.auth.read().token.clone()
    }

    pub fn server_url(&self) -> String {
        self.settings.read().server_url.clone()
    }
}

pub async fn boot(state: &AppState) -> Result<()> {
    // Load persisted settings
    let cfg_path = state.data_dir.join("settings.json");
    if cfg_path.exists() {
        let txt = tokio::fs::read_to_string(&cfg_path).await.context("read settings")?;
        if let Ok(s) = serde_json::from_str::<Settings>(&txt) {
            *state.settings.write() = s;
        }
    }

    // Load persisted auth (raw on disk; ok for desktop usage on user's machine)
    let auth_path = state.data_dir.join("auth.json");
    if auth_path.exists() {
        let txt = tokio::fs::read_to_string(&auth_path).await.context("read auth")?;
        if let Ok(a) = serde_json::from_str::<AuthStore>(&txt) {
            *state.auth.write() = a;
        }
    }

    // Start background loops
    crate::monitor::start_monitors(state.clone()).await;
    crate::sync::start_sync_loop(state.clone()).await;
    crate::monitor::register_hotkeys(state.clone()).await;

    Ok(())
}

pub async fn save_settings_to_disk(state: &AppState) -> Result<()> {
    let s = state.settings.read().clone();
    let txt = serde_json::to_string_pretty(&s)?;
    tokio::fs::write(state.data_dir.join("settings.json"), txt).await?;
    Ok(())
}

pub async fn save_auth_to_disk(state: &AppState) -> Result<()> {
    let a = state.auth.read().clone();
    let txt = serde_json::to_string_pretty(&a)?;
    tokio::fs::write(state.data_dir.join("auth.json"), txt).await?;
    Ok(())
}

pub fn emit_status(state: &AppState) {
    let s = state.session.read().clone();
    let _ = state.app.emit("vkc://status", s);
}
