use serde::{Deserialize, Serialize};

const DEFAULT_SERVER: &str = "https://vongkimco.hoctuthien.com";

/// Local-only settings stored per machine in `app_data_dir/settings.json`.
/// Fields here are *not* governed by the server-side policy because they
/// either need to work before the first network call (server_url) or are
/// inherently per-machine (hotkeys, OS autostart).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub server_url: String,
    pub hotkey_start: String,
    pub hotkey_stop: String,
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server_url: DEFAULT_SERVER.to_string(),
            hotkey_start: "CmdOrCtrl+Alt+S".into(),
            hotkey_stop: "CmdOrCtrl+Alt+E".into(),
            autostart: false,
        }
    }
}
