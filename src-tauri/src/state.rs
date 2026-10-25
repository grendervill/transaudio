use std::path::PathBuf;
use std::sync::Mutex;

use crate::audio::AudioEngine;
use crate::config::SettingsStore;
use crate::diarize::DiarizationManager;
use crate::model::ModelManager;
use crate::transcribe::QueueRuntime;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub app_data: PathBuf,
    pub models: PathBuf,
    pub bin: PathBuf,
    pub cache: PathBuf,
}

impl AppPaths {
    pub fn new(app_data: PathBuf) -> Self {
        Self {
            models: app_data.join("models"),
            bin: app_data.join("bin"),
            cache: app_data.join("cache"),
            app_data,
        }
    }
}

pub struct AppState {
    pub settings: Mutex<SettingsStore>,
    pub paths: AppPaths,
    pub model: std::sync::Arc<ModelManager>,
    pub audio: std::sync::Arc<AudioEngine>,
    pub queue: std::sync::Arc<QueueRuntime>,
    pub diarization: std::sync::Arc<DiarizationManager>,
}

impl AppState {
    pub fn new(
        settings: SettingsStore,
        paths: AppPaths,
        model: std::sync::Arc<ModelManager>,
        audio: std::sync::Arc<AudioEngine>,
        queue: std::sync::Arc<QueueRuntime>,
        diarization: std::sync::Arc<DiarizationManager>,
    ) -> Self {
        Self {
            settings: Mutex::new(settings),
            paths,
            model,
            audio,
            queue,
            diarization,
        }
    }
}
