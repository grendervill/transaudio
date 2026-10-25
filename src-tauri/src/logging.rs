use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// Small append-only diagnostic log. It exists because release builds on
/// Windows have no console: a silent failure must leave a trace on disk.
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn init(path: PathBuf) {
    let _ = LOG_PATH.set(path);
}

pub fn log(message: impl AsRef<str>) {
    let Some(path) = LOG_PATH.get() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    rotate_if_needed(path);
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown-time".into());
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{timestamp}] {}", message.as_ref());
    }
}

fn rotate_if_needed(path: &Path) {
    let too_large = std::fs::metadata(path)
        .map(|meta| meta.len() > 1_048_576)
        .unwrap_or(false);
    if !too_large {
        return;
    }
    let rotated = path.with_extension("log.old");
    let _ = std::fs::remove_file(&rotated);
    let _ = std::fs::rename(path, rotated);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_lines_and_rotates_at_limit() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("transaudio.log");
        let _ = LOG_PATH.set(path.clone());

        log("first line");
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("first line"));

        std::fs::write(&path, "x".repeat(1_048_600)).unwrap();
        log("after rotation");
        assert!(path.with_extension("log.old").is_file());
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("after rotation"));
    }
}
