#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

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
    fn rejects_output_path_that_is_a_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("settings.json");
        let file_path = temp.path().join("not-a-folder");
        fs::write(&file_path, b"x").unwrap();

        let error = SettingsStore::load_or_default(&config_path, file_path).unwrap_err();
        assert!(error.to_string().contains("не является папкой"));
    }

    #[test]
    fn creates_custom_output_directory_when_changed() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("settings.json");
        let mut store =
            SettingsStore::load_or_default(&config_path, temp.path().join("out")).unwrap();
        let custom = PathBuf::from(temp.path().join("Новая папка"));

        store.set_output_dir(custom.clone()).unwrap();

        assert!(custom.is_dir());
        assert_eq!(store.settings().output_dir, custom);
    }
}
