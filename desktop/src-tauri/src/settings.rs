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
    /// "auto" (follow OS preference), "light", or "dark". `serde(default)` keeps
    /// older settings.json files (written before this field existed) loadable.
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "auto".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server_url: DEFAULT_SERVER.to_string(),
            hotkey_start: "CmdOrCtrl+Alt+S".into(),
            hotkey_stop: "CmdOrCtrl+Alt+E".into(),
            autostart: false,
            theme: default_theme(),
        }
    }
}
