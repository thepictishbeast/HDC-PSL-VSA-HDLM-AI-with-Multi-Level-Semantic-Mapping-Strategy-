// PlausiDen AI Desktop Client — Tauri 2.0
// Supports X11 and Wayland via webkit2gtk
// Auto-updates from GitHub Releases

use tauri::Manager;
use tauri_plugin_updater::UpdaterExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Enable logging in debug mode
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Check for updates on startup (non-blocking)
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match handle.updater().expect("updater plugin").check().await {
                    Ok(Some(update)) => {
                        log::info!(
                            "Update available: {} -> {}",
                            env!("CARGO_PKG_VERSION"),
                            update.version
                        );
                        // Download and install
                        if let Err(e) = update.download_and_install(|_, _| {}, || {}).await {
                            log::warn!("Update failed: {}", e);
                        }
                    }
                    Ok(None) => log::info!("No updates available"),
                    Err(e) => log::warn!("Update check failed: {}", e),
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running PlausiDen AI desktop");
}
