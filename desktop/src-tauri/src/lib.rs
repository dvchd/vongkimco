mod auth;
mod commands;
mod db;
mod monitor;
mod policy;
mod screenshot;
mod settings;
mod state;
mod sync;

use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

pub fn run() {
    let _ = env_logger::try_init();

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ));

    builder = builder
        .setup(|app| {
            let handle = app.handle().clone();
            let app_state = state::AppState::new(handle.clone());
            app.manage(app_state.clone());

            // Boot: load settings, try to restore session from keyring, and
            // start background tasks. try_restore_session is awaited inside
            // boot() so the UI sees the rehydrated user as soon as it asks.
            tauri::async_runtime::spawn(async move {
                if let Err(e) = state::boot(&app_state).await {
                    log::error!("boot error: {e:?}");
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_policy,
            commands::refresh_policy,
            commands::set_server_url,
            commands::test_server,
            commands::auth_start,
            commands::auth_poll,
            commands::auth_cancel,
            commands::get_current_user,
            commands::get_auth_status,
            commands::get_device_fingerprint,
            commands::logout,
            commands::start_session,
            commands::stop_session,
            commands::sync_now,
            commands::get_session_state,
            commands::list_local_sessions,
        ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
