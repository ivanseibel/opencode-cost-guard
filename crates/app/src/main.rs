use std::sync::Mutex;

use cost_guard_app::{app_state::AppState, menu, settings, sync_job, timer, tray};
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(autostart_plugin())
        .invoke_handler(tauri::generate_handler![
            settings::settings_get,
            settings::settings_set
        ])
        .on_menu_event(menu::on_menu_event)
        .on_window_event(|window, event| {
            // Close hides the settings window; Quit remains the only exit path.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && window.label() == menu::SETTINGS_LABEL
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let state = AppState::load(data_dir)?;

            app.manage(Mutex::new(state));
            app.manage(sync_job::SyncGuard::default());
            app.manage(tray::LastPresentation::default());
            tray::build(app.handle())?;
            timer::spawn(app.handle());

            // Debug-only smoke fixture: open Settings without driving the tray,
            // which requires Accessibility permission the host may not grant.
            #[cfg(debug_assertions)]
            if std::env::var_os("COST_GUARD_OPEN_SETTINGS").is_some() {
                menu::show_settings(app.handle());
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build Cost Guard")
        .run(|_, event| {
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}

/// Autostart plugin with a stable LaunchAgent name (PLAT-02).
fn autostart_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let builder = tauri_plugin_autostart::Builder::new().app_name(settings::AUTOSTART_APP_NAME);
    #[cfg(target_os = "macos")]
    let builder = builder.macos_launcher(tauri_plugin_autostart::MacosLauncher::LaunchAgent);
    builder.build()
}
