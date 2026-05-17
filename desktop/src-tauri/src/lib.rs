mod commands;
mod db;
mod monitor;
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

            // Boot: load settings + persisted auth + start background tasks
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
            commands::set_server_url,
            commands::test_server,
            commands::device_link_start,
            commands::device_link_poll,
            commands::get_current_user,
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
