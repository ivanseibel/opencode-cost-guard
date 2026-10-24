use std::sync::Mutex;

use cost_guard_app::{app_state::AppState, menu, sync_job, timer, tray};
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .on_menu_event(menu::on_menu_event)
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
