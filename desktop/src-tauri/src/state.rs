use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::auth::PendingLogin;
use crate::db::LocalDb;
use crate::policy::Policy;
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

/// Auth state held entirely in memory. The refresh token (the bit that
/// matters for re-login) lives in the OS keyring, never in this struct.
#[derive(Clone, Debug, Default)]
pub struct AuthStore {
    pub access_token: Option<String>,
    /// Unix-seconds when `access_token` stops being valid. 0 if no token.
    pub access_expires_at: i64,
    pub user: Option<UserInfo>,
}

/// In-memory cross-cutting application state. Wrap in Arc<...> for cheap cloning.
#[derive(Clone)]
pub struct AppState {
    pub app: AppHandle,
    pub data_dir: PathBuf,
    pub screenshot_dir: PathBuf,
    pub db: Arc<LocalDb>,
    pub settings: Arc<RwLock<Settings>>,
    pub policy: Arc<RwLock<Policy>>,
    pub auth: Arc<RwLock<AuthStore>>,
    pub session: Arc<RwLock<SessionState>>,
    pub pending_login: Arc<RwLock<Option<PendingLogin>>>,
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
            policy: Arc::new(RwLock::new(Policy::default())),
            auth: Arc::new(RwLock::new(AuthStore::default())),
            session: Arc::new(RwLock::new(SessionState::initial())),
            pending_login: Arc::new(RwLock::new(None)),
        }
    }

    pub fn server_url(&self) -> String {
        self.settings.read().server_url.clone()
    }
}

pub async fn boot(state: &AppState) -> Result<()> {
    // Load persisted settings. Old settings.json may have policy fields
    // (capture_screenshots, intervals, …) embedded — serde ignores unknown
    // fields by default so those silently drop.
    let cfg_path = state.data_dir.join("settings.json");
    if cfg_path.exists() {
        let txt = tokio::fs::read_to_string(&cfg_path)
            .await
            .context("read settings")?;
        if let Ok(s) = serde_json::from_str::<Settings>(&txt) {
            *state.settings.write() = s;
        }
    }

    // Load cached policy (server-controlled knobs). If absent, the in-memory
    // Policy::default() is used until the first network fetch succeeds.
    *state.policy.write() = crate::policy::load_from_disk(state).await;

    // Best-effort migration: nuke the old plain-text auth.json so leftover
    // tokens from the previous device-code build can't be picked up.
    let legacy_auth = state.data_dir.join("auth.json");
    if legacy_auth.exists() {
        let _ = tokio::fs::remove_file(&legacy_auth).await;
    }

    // Try to restore login from the keyring. Network failures here don't
    // count as logout — we keep the refresh token and try again later.
    let _ = crate::auth::try_restore_session(state).await;

    // Start background loops.
    crate::monitor::start_monitors(state.clone()).await;
    crate::sync::start_sync_loop(state.clone()).await;
    crate::policy::start_refresh_loop(state.clone()).await;
    crate::monitor::register_hotkeys(state.clone()).await;

    // Signal the frontend that boot (including session restore) is complete.
    // Without this, the UI reads user=null before try_restore_session finishes
    // and routes to the login screen on every launch.
    let _ = state.app.emit("vkc://booted", ());

    Ok(())
}

pub async fn save_settings_to_disk(state: &AppState) -> Result<()> {
    let s = state.settings.read().clone();
    let txt = serde_json::to_string_pretty(&s)?;
    tokio::fs::write(state.data_dir.join("settings.json"), txt).await?;
    Ok(())
}

pub fn emit_status(state: &AppState) {
    let s = state.session.read().clone();
    let _ = state.app.emit("vkc://status", s);
}
