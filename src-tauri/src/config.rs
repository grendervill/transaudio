use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    #[default]
    Md,
    Srt,
    Txt,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Md => "md",
            Self::Srt => "srt",
            Self::Txt => "txt",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub output_dir: PathBuf,
    pub model_consent: bool,
    pub export_format: ExportFormat,
    #[serde(default = "default_model_id")]
    pub model_id: String,
    #[serde(default = "default_canary_language")]
    pub canary_language: String,
    #[serde(default)]
    pub diarization: bool,
    #[serde(default = "default_scenario")]
    pub diarization_scenario: String,
    /// Exact number of speakers the user expects; 0 means automatic.
    #[serde(default)]
    pub diarization_speakers: u32,
}

fn default_model_id() -> String {
    "parakeet-tdt-0.6b-v3".into()
}

fn default_canary_language() -> String {
    "ru".into()
}

fn default_scenario() -> String {
    "interview".into()
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("не удалось создать папку результатов: {0}")]
    CreateOutputDir(#[from] std::io::Error),
    #[error("путь результатов не является папкой: {0}")]
    NotADirectory(PathBuf),
    #[error("не удалось сохранить настройки: {0}")]
    Save(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
    settings: Settings,
}

impl SettingsStore {
    pub fn load_or_default(
        config_path: &Path,
        default_output_dir: PathBuf,
    ) -> Result<Self, ConfigError> {
        let settings = fs::read_to_string(config_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Settings>(&raw).ok())
            .unwrap_or(Settings {
                output_dir: default_output_dir,
                model_consent: false,
                export_format: ExportFormat::default(),
                model_id: default_model_id(),
                canary_language: default_canary_language(),
                diarization: false,
                diarization_scenario: default_scenario(),
                diarization_speakers: 0,
            });

        let _ = ensure_output_dir(&settings.output_dir);
        let store = Self {
            path: config_path.to_path_buf(),
            settings,
        };
        if !config_path.exists() {
            store.save()?;
        }
        Ok(store)
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.settings)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn set_output_dir(&mut self, path: PathBuf) -> Result<(), ConfigError> {
        ensure_output_dir(&path)?;
        self.settings.output_dir = path;
        self.save()
    }

    pub fn set_model_consent(&mut self, consent: bool) -> Result<(), ConfigError> {
        self.settings.model_consent = consent;
        self.save()
    }

    pub fn set_export_format(&mut self, format: ExportFormat) -> Result<(), ConfigError> {
        self.settings.export_format = format;
        self.save()
    }

    pub fn set_model_id(&mut self, model_id: String) -> Result<(), ConfigError> {
        self.settings.model_id = model_id;
        self.save()
    }

    pub fn set_canary_language(&mut self, language: String) -> Result<(), ConfigError> {
        self.settings.canary_language = language;
        self.save()
    }

    pub fn set_diarization(
        &mut self,
        enabled: bool,
        scenario: String,
        speakers: u32,
    ) -> Result<(), ConfigError> {
        self.settings.diarization = enabled;
        self.settings.diarization_scenario = scenario;
        self.settings.diarization_speakers = speakers;
        self.save()
    }
}

fn ensure_output_dir(path: &Path) -> Result<(), ConfigError> {
    if path.exists() && !path.is_dir() {
        return Err(ConfigError::NotADirectory(path.to_path_buf()));
    }
    fs::create_dir_all(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    use super::{ExportFormat, SettingsStore};

    #[test]
    fn creates_default_settings_and_output_directory() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("settings.json");
        let output_dir = temp.path().join("Documents").join("Transaudio");

        let store = SettingsStore::load_or_default(&config_path, output_dir.clone()).unwrap();

        assert_eq!(store.settings().output_dir, output_dir);
        assert_eq!(store.settings().export_format, ExportFormat::Md);
        assert!(!store.settings().model_consent);
        assert!(output_dir.is_dir());
        assert!(config_path.is_file());
    }

    #[test]
    fn persists_model_consent_and_format() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("settings.json");
        let first = SettingsStore::load_or_default(&config_path, temp.path().join("out")).unwrap();

        let mut store = first;
        store.set_model_consent(true).unwrap();
        store.set_export_format(ExportFormat::Srt).unwrap();

        let reloaded =
            SettingsStore::load_or_default(&config_path, temp.path().join("fallback")).unwrap();
        assert!(reloaded.settings().model_consent);
        assert_eq!(reloaded.settings().export_format, ExportFormat::Srt);
    }

    #[test]
    fn rejects_output_path_that_is_a_file_when_changing_it() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("settings.json");
        let file_path = temp.path().join("not-a-folder");
        fs::write(&file_path, b"x").unwrap();
        let mut store =
            SettingsStore::load_or_default(&config_path, temp.path().join("out")).unwrap();

        let error = store.set_output_dir(file_path).unwrap_err();
        assert!(error.to_string().contains("не является папкой"));
    }

    #[test]
    fn loads_when_saved_output_folder_is_unavailable() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("settings.json");
        let unavailable = temp.path().join("gone").join("Transaudio");
        fs::write(
            &config_path,
            format!(
                r#"{{"output_dir":"{}","model_consent":true,"export_format":"md"}}"#,
                unavailable.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .unwrap();
        // Make the path invalid (a file at the parent position).
        fs::write(temp.path().join("gone"), b"x").unwrap();

        let store = SettingsStore::load_or_default(&config_path, temp.path().join("fallback"))
            .expect("app must be able to start with a stale output folder");
        assert_eq!(store.settings().output_dir, unavailable);
        assert!(store.settings().model_consent);
    }

    #[test]
    fn creates_custom_output_directory_when_changed() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("settings.json");
        let mut store =
            SettingsStore::load_or_default(&config_path, temp.path().join("out")).unwrap();
        let custom = temp.path().join("Новая папка");

        store.set_output_dir(custom.clone()).unwrap();

        assert!(custom.is_dir());
        assert_eq!(store.settings().output_dir, custom);
    }
    #[test]
    fn migrates_settings_without_model_fields() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("settings.json");
        fs::write(
            &config_path,
            r#"{"output_dir":"/tmp/transaudio","model_consent":true,"export_format":"md"}"#,
        )
        .unwrap();

        let store =
            SettingsStore::load_or_default(&config_path, temp.path().join("fallback")).unwrap();
        let settings = store.settings();

        assert_eq!(settings.model_id, "parakeet-tdt-0.6b-v3");
        assert_eq!(settings.canary_language, "ru");
        assert!(!settings.diarization);
        assert_eq!(settings.diarization_scenario, "interview");
        assert_eq!(settings.diarization_speakers, 0);
    }

    #[test]
    fn persists_model_and_diarization_settings() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("settings.json");
        let mut store =
            SettingsStore::load_or_default(&config_path, temp.path().join("out")).unwrap();
        store.set_model_id("gigaam-v3-e2e-rnnt".into()).unwrap();
        store.set_canary_language("en".into()).unwrap();
        store.set_diarization(true, "meeting".into(), 3).unwrap();

        let reloaded =
            SettingsStore::load_or_default(&config_path, temp.path().join("fallback")).unwrap();
        assert_eq!(reloaded.settings().model_id, "gigaam-v3-e2e-rnnt");
        assert_eq!(reloaded.settings().canary_language, "en");
        assert!(reloaded.settings().diarization);
        assert_eq!(reloaded.settings().diarization_scenario, "meeting");
        assert_eq!(reloaded.settings().diarization_speakers, 3);
    }
}
