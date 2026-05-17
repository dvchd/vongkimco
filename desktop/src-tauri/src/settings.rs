use serde::{Deserialize, Serialize};

const DEFAULT_SERVER: &str = "https://vongkimco.hoctuthien.com";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub server_url: String,
    pub capture_screenshots: bool,
    pub screenshot_interval_secs: u64,
    pub activity_sample_interval_secs: u64,
    pub app_snapshot_interval_secs: u64,
    pub idle_threshold_secs: u64,
    pub hotkey_start: String,
    pub hotkey_stop: String,
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server_url: DEFAULT_SERVER.to_string(),
            capture_screenshots: true,
            screenshot_interval_secs: 180,
            activity_sample_interval_secs: 30,
            app_snapshot_interval_secs: 60,
            idle_threshold_secs: 120,
            hotkey_start: "CmdOrCtrl+Alt+S".into(),
            hotkey_stop: "CmdOrCtrl+Alt+E".into(),
            autostart: false,
        }
    }
}
