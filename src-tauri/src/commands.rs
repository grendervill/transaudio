use std::path::PathBuf;

use tauri::State;

use crate::config::Settings;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let store = state.settings.lock().map_err(|_| "Настройки недоступны")?;
    Ok(store.settings().clone())
}

#[tauri::command]
pub fn set_output_dir(path: String, state: State<'_, AppState>) -> Result<Settings, String> {
    let mut store = state.settings.lock().map_err(|_| "Настройки недоступны")?;
    store
        .set_output_dir(PathBuf::from(path))
        .map_err(|error| error.to_string())?;
    Ok(store.settings().clone())
}

#[tauri::command]
pub fn get_model_status(state: State<'_, AppState>) -> crate::model::ModelStatus {
    state.model.status()
}

#[tauri::command]
pub fn grant_model_consent(
    state: State<'_, AppState>,
) -> Result<crate::model::ModelStatus, String> {
    {
        let mut store = state.settings.lock().map_err(|_| "Настройки недоступны")?;
        store
            .set_model_consent(true)
            .map_err(|error| error.to_string())?;
    }
    state
        .model
        .grant_and_download()
        .map_err(|error| error.to_string())?;
    Ok(state.model.status())
}

#[tauri::command]
pub fn download_model(state: State<'_, AppState>) -> Result<crate::model::ModelStatus, String> {
    state.model.retry().map_err(|error| error.to_string())?;
    Ok(state.model.status())
}

#[tauri::command]
pub fn inspect_files(paths: Vec<String>) -> crate::audio::InspectFilesResult {
    crate::audio::inspect_files(&paths)
}

#[derive(serde::Serialize)]
pub struct BootstrapPayload {
    pub settings: Settings,
    pub model: crate::model::ModelStatus,
    pub queue: crate::transcribe::QueueState,
}

#[tauri::command]
pub fn bootstrap(state: State<'_, AppState>) -> Result<BootstrapPayload, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Настройки недоступны")?
        .settings()
        .clone();
    Ok(BootstrapPayload {
        settings,
        model: state.model.status(),
        queue: state.queue.state(),
    })
}

#[tauri::command]
pub fn start_transcription(
    app: tauri::AppHandle,
    files: Vec<crate::transcribe::FileJob>,
    state: State<'_, AppState>,
) -> Result<crate::transcribe::QueueState, String> {
    if !state.model.is_ready() {
        return Err("Дождитесь загрузки модели Parakeet V3".into());
    }
    if files.is_empty() {
        return Err("Добавьте хотя бы один файл".into());
    }
    let output_dir = state
        .settings
        .lock()
        .map_err(|_| "Настройки недоступны")?
        .settings()
        .output_dir
        .clone();
    let queue_state = state.queue.start(files.len())?;
    let runtime = std::sync::Arc::clone(&state.queue);
    let model = std::sync::Arc::clone(&state.model);
    let audio = std::sync::Arc::clone(&state.audio);
    let cache_dir = state.paths.cache.clone();
    let app_handle = app.clone();
    let _ = tauri::Emitter::emit(&app, "queue-state", queue_state.clone());
    std::thread::spawn(move || {
        crate::transcribe::run_batch(
            app_handle, model, audio, cache_dir, output_dir, files, runtime,
        );
    });
    Ok(queue_state)
}

#[tauri::command]
pub fn set_paused(
    app: tauri::AppHandle,
    paused: bool,
    state: State<'_, AppState>,
) -> Result<crate::transcribe::QueueState, String> {
    let snapshot = state.queue.set_paused(paused)?;
    let _ = tauri::Emitter::emit(&app, "queue-state", snapshot.clone());
    Ok(snapshot)
}

#[tauri::command]
pub fn get_queue_state(state: State<'_, AppState>) -> crate::transcribe::QueueState {
    state.queue.state()
}
