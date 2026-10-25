use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use serde::Serialize;
use uuid::Uuid;

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp3", "wav", "m4a", "flac", "aac", "ogg", "opus", "mp4", "mov", "mkv", "webm",
];

pub fn read_wav_samples(path: &Path) -> Result<Vec<f32>, String> {
    transcribe_rs::audio::read_wav_samples(path).map_err(|error| error.to_string())
}

pub struct AudioEngine {
    bin_dir: PathBuf,
    ffmpeg: Mutex<Option<PathBuf>>,
}

impl AudioEngine {
    pub fn new(bin_dir: PathBuf) -> Self {
        Self {
            bin_dir,
            ffmpeg: Mutex::new(None),
        }
    }

    pub fn bin_dir(&self) -> &Path {
        &self.bin_dir
    }

    fn cached_ffmpeg(&self) -> PathBuf {
        self.bin_dir.join(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        })
    }

    fn system_ffmpeg() -> Option<PathBuf> {
        let mut command = create_command("ffmpeg");
        command.arg("-version");
        let output = command.output().ok()?;
        output.status.success().then(|| PathBuf::from("ffmpeg"))
    }

    pub fn locate_ffmpeg(&self) -> Option<PathBuf> {
        if let Ok(cached) = self.ffmpeg.lock() {
            if let Some(path) = cached.as_ref() {
                return Some(path.clone());
            }
        }

        let local = self.cached_ffmpeg();
        let found = if local.is_file() {
            Some(local)
        } else {
            Self::system_ffmpeg()
        };

        if let Some(path) = &found {
            if let Ok(mut cached) = self.ffmpeg.lock() {
                *cached = Some(path.clone());
            }
        }
        found
    }

    pub fn ensure_ffmpeg(&self) -> Result<PathBuf, String> {
        if let Some(path) = self.locate_ffmpeg() {
            return Ok(path);
        }

        std::fs::create_dir_all(&self.bin_dir)
            .map_err(|error| format!("Не удалось создать папку для FFmpeg: {error}"))?;
        let url = ffmpeg_sidecar::download::ffmpeg_download_url()
            .map_err(|error| format!("Не удалось определить сборку FFmpeg: {error}"))?;
        let archive = ffmpeg_sidecar::download::download_ffmpeg_package(&url, &self.bin_dir)
            .map_err(|error| format!("Не удалось скачать FFmpeg: {error}"))?;
        ffmpeg_sidecar::download::unpack_ffmpeg_without_extras(&archive, &self.bin_dir)
            .map_err(|error| format!("Не удалось распаковать FFmpeg: {error}"))?;
        let _ = std::fs::remove_file(archive);

        let binary = self.cached_ffmpeg();
        if !binary.is_file() {
            return Err("FFmpeg не найден после установки".into());
        }
        if let Ok(mut cached) = self.ffmpeg.lock() {
            *cached = Some(binary.clone());
        }
        Ok(binary)
    }

    pub fn extract_wav(&self, input: &Path, output: &Path) -> Result<(), String> {
        let ffmpeg = self.ensure_ffmpeg()?;
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Не удалось создать временную папку: {error}"))?;
        }
        let mut command = create_command(&ffmpeg);
        command
            .arg("-nostdin")
            .arg("-y")
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-i")
            .arg(input)
            .arg("-vn")
            .arg("-ac")
            .arg("1")
            .arg("-ar")
            .arg("16000")
            .arg("-c:a")
            .arg("pcm_s16le")
            .arg(output);
        let result = command
            .output()
            .map_err(|error| format!("Не удалось запустить FFmpeg: {error}"))?;
        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let message = stderr.trim();
            return Err(if message.is_empty() {
                "FFmpeg не смог извлечь аудио".into()
            } else {
                format!("FFmpeg: {message}")
            });
        }
        Ok(())
    }
}

fn create_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut command = command;
        command.creation_flags(0x08000000);
        return command;
    }
    #[cfg(not(windows))]
    {
        command
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InspectedFile {
    pub id: String,
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RejectedFile {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct InspectFilesResult {
    pub files: Vec<InspectedFile>,
    pub rejected: Vec<RejectedFile>,
}

pub fn inspect_file(path: &Path) -> Result<InspectedFile, String> {
    if !path.exists() {
        return Err("Файл не найден".into());
    }
    if !path.is_file() {
        return Err("Это не файл".into());
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_lowercase)
        .ok_or_else(|| "У файла нет расширения".to_string())?;
    if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(format!("Формат .{extension} не поддерживается"));
    }
    let size_bytes = path
        .metadata()
        .map_err(|error| format!("Не удалось прочитать файл: {error}"))?
        .len();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Без имени")
        .to_string();

    Ok(InspectedFile {
        id: Uuid::new_v4().to_string(),
        path: path.to_string_lossy().to_string(),
        name,
        extension: extension.to_uppercase(),
        size_bytes,
    })
}

pub fn inspect_files(paths: &[String]) -> InspectFilesResult {
    let mut result = InspectFilesResult::default();
    for path in paths {
        match inspect_file(Path::new(path)) {
            Ok(file) => result.files.push(file),
            Err(reason) => result.rejected.push(RejectedFile {
                path: path.clone(),
                reason,
            }),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_fake_media(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn accepts_supported_extension_case_insensitively() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = write_fake_media(temp.path(), "запись.MP4", b"video");

        let inspected = inspect_file(&path).unwrap();

        assert_eq!(inspected.extension, "MP4");
        assert_eq!(inspected.size_bytes, 5);
        assert_eq!(inspected.name, "запись.MP4");
        assert_eq!(inspected.path, path.to_string_lossy());
        assert!(!inspected.id.is_empty());
    }

    #[test]
    fn rejects_unsupported_and_missing_files_with_reasons() {
        let temp = tempfile::TempDir::new().unwrap();
        let unsupported = write_fake_media(temp.path(), "notes.txt", b"x");
        let missing = temp.path().join("missing.mp3");

        let unsupported_error = inspect_file(&unsupported).unwrap_err();
        let missing_error = inspect_file(&missing).unwrap_err();

        assert!(unsupported_error.contains("не поддерживается"));
        assert_eq!(missing_error, "Файл не найден");
    }

    #[test]
    fn preserves_order_and_collects_rejections() {
        let temp = tempfile::TempDir::new().unwrap();
        let first = write_fake_media(temp.path(), "first.mp3", b"1");
        let broken = write_fake_media(temp.path(), "bad.xyz", b"2");
        let second = write_fake_media(temp.path(), "second.m4a", b"3");
        let paths = vec![
            first.to_string_lossy().to_string(),
            broken.to_string_lossy().to_string(),
            second.to_string_lossy().to_string(),
        ];

        let result = inspect_files(&paths);

        assert_eq!(result.files.len(), 2);
        assert_eq!(result.rejected.len(), 1);
        assert_eq!(result.files[0].name, "first.mp3");
        assert_eq!(result.files[1].name, "second.m4a");
        assert_eq!(result.rejected[0].reason, "Формат .xyz не поддерживается");
    }

    #[test]
    fn generates_unique_ids() {
        let temp = tempfile::TempDir::new().unwrap();
        let first = inspect_file(&write_fake_media(temp.path(), "a.mp3", b"a")).unwrap();
        let second = inspect_file(&write_fake_media(temp.path(), "b.mp3", b"b")).unwrap();
        assert_ne!(first.id, second.id);
    }
    #[test]
    fn prefers_cached_ffmpeg_binary() {
        let temp = tempfile::TempDir::new().unwrap();
        let engine = AudioEngine::new(temp.path().join("bin"));
        std::fs::create_dir_all(engine.bin_dir()).unwrap();
        let cached = engine.bin_dir().join(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });
        std::fs::write(&cached, b"binary").unwrap();

        assert_eq!(engine.locate_ffmpeg(), Some(cached));
    }

    #[test]
    fn reads_16khz_mono_pcm_wav() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("sample.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for sample in [0_i16, 8000, -8000, 16000] {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();

        let samples = read_wav_samples(&path).unwrap();
        assert_eq!(samples.len(), 4);
        assert!((samples[1] - 0.244).abs() < 0.01);
    }

    #[test]
    fn extracts_audio_to_16khz_mono_when_ffmpeg_is_available() {
        let temp = tempfile::TempDir::new().unwrap();
        let engine = AudioEngine::new(temp.path().join("bin"));
        if engine.locate_ffmpeg().is_none() {
            eprintln!("skipping ffmpeg extraction test: ffmpeg is not installed");
            return;
        }

        let source_path = temp.path().join("source.wav");
        let source_spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&source_path, source_spec).unwrap();
        for index in 0..44_100 {
            let sample = ((index as f32 * 0.05).sin() * 4000.0) as i16;
            writer.write_sample(sample).unwrap();
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();

        let output = temp.path().join("extracted.wav");
        engine.extract_wav(&source_path, &output).unwrap();

        let spec = hound::WavReader::open(&output).unwrap().spec();
        assert_eq!(spec.sample_rate, 16_000);
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.bits_per_sample, 16);
        assert!(!read_wav_samples(&output).unwrap().is_empty());
    }
}
