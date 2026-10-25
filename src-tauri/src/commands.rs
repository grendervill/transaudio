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
    let mut store = state
        .settings
        .lock()
        .map_err(|_| "Настройки недоступны")?;
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
pub fn grant_model_consent(state: State<'_, AppState>) -> Result<crate::model::ModelStatus, String> {
    {
        let mut store = state
            .settings
            .lock()
            .map_err(|_| "Настройки недоступны")?;
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
    state
        .model
        .retry()
        .map_err(|error| error.to_string())?;
    Ok(state.model.status())
}

#[tauri::command]
pub fn inspect_files(paths: Vec<String>) -> crate::audio::InspectFilesResult {
    crate::audio::inspect_files(&paths)
}
