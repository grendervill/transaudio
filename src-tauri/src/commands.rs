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
