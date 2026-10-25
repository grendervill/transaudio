use std::path::PathBuf;
use std::sync::Mutex;

use crate::config::SettingsStore;

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
}

impl AppState {
    pub fn new(settings: SettingsStore, paths: AppPaths) -> Self {
        Self {
            settings: Mutex::new(settings),
            paths,
        }
    }
}
