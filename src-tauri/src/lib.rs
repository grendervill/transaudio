use std::fs;

use tauri::Manager;

use crate::config::SettingsStore;
use crate::state::{AppPaths, AppState};

pub mod commands;
pub mod config;
pub mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            let documents = app.path().document_dir()?;
            fs::create_dir_all(&app_data)?;

            let paths = AppPaths::new(app_data.clone());
            fs::create_dir_all(&paths.models)?;
            fs::create_dir_all(&paths.bin)?;
            fs::create_dir_all(&paths.cache)?;

            let settings = SettingsStore::load_or_default(
                &app_data.join("settings.json"),
                documents.join("Transaudio"),
            )?;
            app.manage(AppState::new(settings, paths));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_output_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running Transaudio");
}
