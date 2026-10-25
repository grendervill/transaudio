use std::fs;

use std::sync::Arc;

use tauri::{Emitter, Manager};

use crate::audio::AudioEngine;
use crate::config::SettingsStore;
use crate::model::{ModelManager, ModelPaths, StatusSink};
use crate::state::{AppPaths, AppState};
use crate::transcribe::QueueRuntime;

pub mod audio;
pub mod commands;
pub mod config;
pub mod export;
pub mod logging;
pub mod model;
pub mod state;
pub mod transcribe;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| match setup_state(app) {
            Ok(()) => Ok(()),
            Err(error) => {
                logging::log(format!("startup failed: {error}"));
                Err(error)
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::inspect_files,
            commands::set_output_dir,
            commands::get_model_status,
            commands::grant_model_consent,
            commands::download_model,
            commands::bootstrap,
            commands::start_transcription,
            commands::set_paused,
            commands::get_queue_state,
            commands::export_results,
            commands::open_output_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running Transaudio");
}

fn setup_state(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_data = app.path().app_data_dir()?;
    fs::create_dir_all(&app_data)?;
    logging::init(app_data.join("transaudio.log"));
    logging::log(format!(
        "start Transaudio {} os={} arch={} data={}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        app_data.display()
    ));

    let documents = app
        .path()
        .document_dir()
        .unwrap_or_else(|_| app_data.join("Documents"));

    let paths = AppPaths::new(app_data.clone());
    fs::create_dir_all(&paths.models)?;
    fs::create_dir_all(&paths.bin)?;
    fs::create_dir_all(&paths.cache)?;

    let settings = SettingsStore::load_or_default(
        &app_data.join("settings.json"),
        documents.join("Transaudio"),
    )?;
    logging::log(format!(
        "settings: consent={} output={}",
        settings.settings().model_consent,
        settings.settings().output_dir.display()
    ));

    let handle = app.handle().clone();
    let sink: StatusSink = Arc::new(move |status| {
        let _ = handle.emit("model-status", status);
    });
    let model_paths = ModelPaths::new(app_data.clone());
    let model = ModelManager::new(model_paths, sink);
    model.start_on_launch(settings.settings().model_consent);

    let audio = Arc::new(AudioEngine::new(paths.bin.clone()));
    let queue = Arc::new(QueueRuntime::default());

    app.manage(AppState::new(settings, paths, model, audio, queue));
    logging::log("startup complete");
    Ok(())
}
