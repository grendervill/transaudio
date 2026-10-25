use std::fs;

use std::sync::Arc;

use tauri::{Emitter, Manager};

use crate::audio::AudioEngine;
use crate::config::SettingsStore;
use crate::diarize::{DiarizationManager, DiarizationSink};
use crate::model::{ModelManager, ModelPaths, StatusSink};
use crate::state::{AppPaths, AppState};
use crate::transcribe::QueueRuntime;

pub mod audio;
pub mod catalog;
pub mod commands;
pub mod config;
pub mod diarize;
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
            commands::list_models,
            commands::select_model,
            commands::delete_model,
            commands::set_canary_language,
            commands::set_diarization,
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
    let selected = settings.settings().model_id.clone();
    let hint = crate::catalog::find(&selected)
        .filter(|spec| spec.needs_language)
        .map(|_| settings.settings().canary_language.clone());
    model.start_on_launch(settings.settings().model_consent, &selected, hint);

    let audio = Arc::new(AudioEngine::new(paths.bin.clone()));
    let queue = Arc::new(QueueRuntime::default());

    let diarization_handle = app.handle().clone();
    let diarization_sink: DiarizationSink = Arc::new(move |status| {
        let _ = diarization_handle.emit("diarization-status", status);
    });
    let diarization = DiarizationManager::new(paths.models.join("diarization"), diarization_sink);
    if diarization.is_ready() {
        logging::log("diarization: models found");
    }

    app.manage(AppState::new(
        settings,
        paths,
        model,
        audio,
        queue,
        diarization,
    ));
    logging::log("startup complete");
    Ok(())
}
