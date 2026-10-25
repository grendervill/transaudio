use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::catalog::{self, ModelSpec};
use crate::config::Settings;
use crate::model::{self, ModelStatus};
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

#[derive(Serialize)]
pub struct ModelCard {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub family: crate::catalog::ModelFamily,
    pub languages: Vec<&'static str>,
    pub language_count: u16,
    pub quality_en: Option<f32>,
    pub quality_ru: Option<f32>,
    pub speed_x: f32,
    pub size_bytes: u64,
    pub license: &'static str,
    pub needs_language: bool,
    pub recommended_for: Vec<&'static str>,
    pub badge: Option<&'static str>,
    pub installed: bool,
    pub selected: bool,
}

fn card_for(spec: &'static ModelSpec, state: &AppState) -> ModelCard {
    ModelCard {
        id: spec.id,
        name: spec.name,
        description: spec.description,
        family: spec.family,
        languages: spec.languages.to_vec(),
        language_count: spec.language_count,
        quality_en: spec.quality_en,
        quality_ru: spec.quality_ru,
        speed_x: spec.speed_x,
        size_bytes: spec.size_bytes,
        license: spec.license,
        needs_language: spec.needs_language,
        recommended_for: spec.recommended_for.to_vec(),
        badge: spec.badge,
        installed: model::is_installed(spec, &state.paths.models),
        selected: state.model.selected_id() == spec.id,
    }
}

fn cards(state: &AppState) -> Vec<ModelCard> {
    catalog::MODELS
        .iter()
        .map(|spec| card_for(spec, state))
        .collect()
}

fn language_for(spec: &ModelSpec, settings: &Settings) -> Option<String> {
    if spec.needs_language {
        Some(settings.canary_language.clone())
    } else {
        None
    }
}

#[tauri::command]
pub fn get_model_status(state: State<'_, AppState>) -> ModelStatus {
    state.model.status()
}

#[tauri::command]
pub fn list_models(state: State<'_, AppState>) -> Vec<ModelCard> {
    cards(&state)
}

#[tauri::command]
pub fn select_model(
    app: tauri::AppHandle,
    id: String,
    language: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ModelCard>, String> {
    crate::logging::log(format!("command: select_model {id}"));
    let spec = catalog::find(&id).ok_or_else(|| format!("Неизвестная модель: {id}"))?;
    if matches!(
        state.queue.state().status,
        crate::transcribe::QueueStatus::Running | crate::transcribe::QueueStatus::Paused
    ) {
        return Err("Дождитесь завершения очереди, чтобы сменить модель.".into());
    }
    {
        let mut store = state.settings.lock().map_err(|_| "Настройки недоступны")?;
        store.set_model_id(id.clone()).map_err(|e| e.to_string())?;
        if let Some(language) = &language {
            if spec.needs_language {
                store
                    .set_canary_language(language.clone())
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    let hint = language_for(spec, &settings_snapshot(&state)?);
    state
        .model
        .select(&id, hint)
        .map_err(|error| error.to_string())?;
    let _ = tauri::Emitter::emit(&app, "model-status", state.model.status());
    Ok(cards(&state))
}

#[tauri::command]
pub fn download_model(
    app: tauri::AppHandle,
    id: String,
    language: Option<String>,
    state: State<'_, AppState>,
) -> Result<ModelStatus, String> {
    crate::logging::log(format!("command: download_model {id}"));
    let spec = catalog::find(&id).ok_or_else(|| format!("Неизвестная модель: {id}"))?;
    {
        let mut store = state.settings.lock().map_err(|_| "Настройки недоступны")?;
        store.set_model_consent(true).map_err(|e| e.to_string())?;
        store.set_model_id(id.clone()).map_err(|e| e.to_string())?;
        if let Some(language) = &language {
            if spec.needs_language {
                store
                    .set_canary_language(language.clone())
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    let hint = language_for(spec, &settings_snapshot(&state)?);
    let status = state
        .model
        .download(&id, hint)
        .map_err(|error| error.to_string())?;
    let _ = tauri::Emitter::emit(&app, "model-status", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn delete_model(id: String, state: State<'_, AppState>) -> Result<Vec<ModelCard>, String> {
    state.model.delete(&id).map_err(|error| error.to_string())?;
    Ok(cards(&state))
}

#[tauri::command]
pub fn set_canary_language(
    language: String,
    state: State<'_, AppState>,
) -> Result<Settings, String> {
    {
        let mut store = state.settings.lock().map_err(|_| "Настройки недоступны")?;
        store
            .set_canary_language(language.clone())
            .map_err(|e| e.to_string())?;
    }
    let settings = settings_snapshot(&state)?;
    let spec = catalog::find(&settings.model_id);
    if spec.map(|spec| spec.needs_language).unwrap_or(false) {
        state.model.set_language(Some(language));
    }
    Ok(settings)
}

#[tauri::command]
pub fn grant_model_consent(
    app: tauri::AppHandle,
    id: Option<String>,
    language: Option<String>,
    state: State<'_, AppState>,
) -> Result<ModelStatus, String> {
    crate::logging::log("command: grant_model_consent");
    let settings = {
        let mut store = state.settings.lock().map_err(|_| "Настройки недоступны")?;
        store.set_model_consent(true).map_err(|e| e.to_string())?;
        let selected = id.unwrap_or_else(|| store.settings().model_id.clone());
        store
            .set_model_id(selected.clone())
            .map_err(|e| e.to_string())?;
        let spec = catalog::find(&selected).ok_or_else(|| "Модель не найдена".to_string())?;
        if spec.needs_language {
            if let Some(language) = &language {
                store
                    .set_canary_language(language.clone())
                    .map_err(|e| e.to_string())?;
            }
        }
        store.settings().clone()
    };
    let spec = catalog::find(&settings.model_id).ok_or_else(|| "Модель не найдена".to_string())?;
    let hint = language_for(spec, &settings);
    let status = state
        .model
        .download(&settings.model_id, hint)
        .map_err(|error| error.to_string())?;
    let _ = tauri::Emitter::emit(&app, "model-status", status.clone());
    Ok(status)
}

fn settings_snapshot(state: &AppState) -> Result<Settings, String> {
    let store = state.settings.lock().map_err(|_| "Настройки недоступны")?;
    Ok(store.settings().clone())
}

#[tauri::command]
pub fn inspect_files(paths: Vec<String>) -> crate::audio::InspectFilesResult {
    crate::audio::inspect_files(&paths)
}

#[derive(Serialize)]
pub struct BootstrapPayload {
    pub settings: Settings,
    pub model: ModelStatus,
    pub selected_model: String,
    pub models: Vec<ModelCard>,
    pub diarization: crate::diarize::DiarizationStatus,
    pub queue: crate::transcribe::QueueState,
}

#[tauri::command]
pub fn bootstrap(state: State<'_, AppState>) -> Result<BootstrapPayload, String> {
    let settings = settings_snapshot(&state)?;
    Ok(BootstrapPayload {
        selected_model: state.model.selected_id(),
        models: cards(&state),
        settings,
        model: state.model.status(),
        diarization: state.diarization.status(),
        queue: state.queue.state(),
    })
}

#[tauri::command]
pub fn set_diarization(
    app: tauri::AppHandle,
    enabled: bool,
    scenario: String,
    state: State<'_, AppState>,
) -> Result<crate::diarize::DiarizationStatus, String> {
    {
        let mut store = state.settings.lock().map_err(|_| "Настройки недоступны")?;
        store
            .set_diarization(enabled, scenario)
            .map_err(|error| error.to_string())?;
    }
    if enabled && !state.diarization.is_ready() {
        state.diarization.start_download();
    }
    let status = state.diarization.status();
    let _ = tauri::Emitter::emit(&app, "diarization-status", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn start_transcription(
    app: tauri::AppHandle,
    files: Vec<crate::transcribe::FileJob>,
    state: State<'_, AppState>,
) -> Result<crate::transcribe::QueueState, String> {
    if !state.model.is_ready() {
        return Err("Дождитесь загрузки выбранной модели".into());
    }
    if files.is_empty() {
        return Err("Добавьте хотя бы один файл".into());
    }
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Настройки недоступны")?
        .settings()
        .clone();
    if settings.diarization && !state.diarization.is_ready() {
        return Err("Модели спикеров ещё не готовы. Дождитесь загрузки.".into());
    }
    let output_dir = settings.output_dir.clone();
    let diarization = if settings.diarization {
        Some(std::sync::Arc::clone(&state.diarization))
    } else {
        None
    };
    let scenario = settings.diarization_scenario.clone();
    let queue_state = state.queue.start(files.len())?;
    let runtime = std::sync::Arc::clone(&state.queue);
    let model = std::sync::Arc::clone(&state.model);
    let audio = std::sync::Arc::clone(&state.audio);
    let cache_dir = state.paths.cache.clone();
    let app_handle = app.clone();
    let _ = tauri::Emitter::emit(&app, "queue-state", queue_state.clone());
    std::thread::spawn(move || {
        crate::transcribe::run_batch(
            app_handle,
            model,
            audio,
            cache_dir,
            output_dir,
            files,
            runtime,
            diarization,
            scenario,
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

#[derive(Serialize)]
pub struct ExportResult {
    pub files_written: Vec<String>,
}

#[tauri::command]
pub fn export_results(
    ids: Vec<String>,
    format: crate::config::ExportFormat,
    state: State<'_, AppState>,
) -> Result<ExportResult, String> {
    let output_dir = settings_snapshot(&state)?.output_dir;
    let results = state
        .queue
        .results
        .lock()
        .map_err(|_| "Расшифровки недоступны")?;
    let written = crate::export::export_completed(&results, &output_dir, &ids, format)?;
    Ok(ExportResult {
        files_written: written
            .into_iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
    })
}

#[tauri::command]
pub fn open_output_dir(state: State<'_, AppState>) -> Result<(), String> {
    let output_dir = settings_snapshot(&state)?.output_dir;
    tauri_plugin_opener::open_path(&output_dir, None::<&str>).map_err(|error| error.to_string())
}
