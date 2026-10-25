use std::fs::File;
use std::io::BufReader;
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
        let archive_name = url
            .split('/')
            .next_back()
            .unwrap_or("ffmpeg-download")
            .split('?')
            .next()
            .unwrap_or("ffmpeg-download");
        let archive_path = self.bin_dir.join(archive_name);
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(20))
            .timeout(std::time::Duration::from_secs(30 * 60))
            .build()
            .map_err(|error| format!("Не удалось подготовить загрузку FFmpeg: {error}"))?;
        let mut response = client
            .get(url)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| format!("Не удалось скачать FFmpeg: {error}"))?;
        let mut file = std::fs::File::create(&archive_path)
            .map_err(|error| format!("Не удалось создать файл FFmpeg: {error}"))?;
        std::io::copy(&mut response, &mut file)
            .map_err(|error| format!("Не удалось сохранить архив FFmpeg: {error}"))?;
        drop(file);
        ffmpeg_sidecar::download::unpack_ffmpeg_without_extras(&archive_path, &self.bin_dir)
            .map_err(|error| format!("Не удалось распаковать FFmpeg: {error}"))?;
        let _ = std::fs::remove_file(archive_path);

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

pub const SAMPLE_RATE: usize = 16_000;
pub const TARGET_CHUNK_SECS: f32 = 30.0;
pub const SEARCH_WINDOW_SECS: f32 = 3.0;
const FRAME_SECS: f32 = 0.03;

#[derive(Debug, Clone)]
pub struct WavChunk {
    pub start_sample: usize,
    pub end_sample: usize,
    pub samples: Vec<f32>,
}

pub struct WavChunker {
    reader: hound::WavReader<BufReader<File>>,
    buffer: Vec<f32>,
    start_sample: usize,
    total_samples: usize,
}

impl WavChunker {
    pub fn open(path: &Path) -> Result<Self, String> {
        let reader = hound::WavReader::open(path)
            .map_err(|error| format!("Не удалось открыть WAV: {error}"))?;
        let spec = reader.spec();
        if spec.channels != 1
            || spec.sample_rate != SAMPLE_RATE as u32
            || spec.bits_per_sample != 16
            || spec.sample_format != hound::SampleFormat::Int
        {
            return Err("Ожидается WAV 16 кГц, mono, 16-bit PCM".into());
        }
        let total_samples = reader.duration() as usize;
        Ok(Self {
            reader,
            buffer: Vec::new(),
            start_sample: 0,
            total_samples,
        })
    }

    pub fn duration_secs(&self) -> f64 {
        self.total_samples as f64 / SAMPLE_RATE as f64
    }

    pub fn total_samples(&self) -> usize {
        self.total_samples
    }

    pub fn next_chunk(&mut self) -> Result<Option<WavChunk>, String> {
        if self.start_sample >= self.total_samples {
            return Ok(None);
        }

        let target = (TARGET_CHUNK_SECS * SAMPLE_RATE as f32) as usize;
        let search = (SEARCH_WINDOW_SECS * SAMPLE_RATE as f32) as usize;
        let frame = (FRAME_SECS * SAMPLE_RATE as f32) as usize;
        let remaining = self.total_samples - self.start_sample;
        let desired = remaining.min(target + search);
        self.fill_to(desired)?;

        if self.buffer.is_empty() {
            return Ok(None);
        }

        let split = if remaining > target + search {
            choose_split_point(&self.buffer, target, search, frame)
        } else {
            self.buffer.len()
        };
        let start_sample = self.start_sample;
        let samples: Vec<f32> = self.buffer.drain(..split).collect();
        self.start_sample += split;
        Ok(Some(WavChunk {
            start_sample,
            end_sample: self.start_sample,
            samples,
        }))
    }

    fn fill_to(&mut self, desired: usize) -> Result<(), String> {
        let needed = desired.saturating_sub(self.buffer.len());
        if needed == 0 {
            return Ok(());
        }
        let mut iterator = self.reader.samples::<i16>();
        for _ in 0..needed {
            match iterator.next() {
                Some(Ok(sample)) => self.buffer.push(sample as f32 / i16::MAX as f32),
                Some(Err(error)) => return Err(format!("Ошибка чтения WAV: {error}")),
                None => break,
            }
        }
        Ok(())
    }
}

fn choose_split_point(buffer: &[f32], target: usize, search: usize, frame: usize) -> usize {
    let low = target.saturating_sub(search);
    let high = (target + search).min(buffer.len());
    let mut best = target.min(buffer.len());
    let mut best_energy = f32::MAX;
    let mut offset = low;
    while offset + frame <= high {
        let energy = buffer[offset..offset + frame]
            .iter()
            .map(|sample| sample * sample)
            .sum::<f32>();
        if energy < best_energy {
            best_energy = energy;
            best = offset + frame;
        }
        offset += frame;
    }
    best
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
    fn write_test_wav(path: &Path, seconds: u32, mut sample_at: impl FnMut(usize) -> i16) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for index in 0..(16_000 * seconds as usize) {
            writer.write_sample(sample_at(index)).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn streams_long_wav_in_bounded_contiguous_chunks() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("long.wav");
        write_test_wav(&path, 65, |_| 3000);

        let mut chunker = WavChunker::open(&path).unwrap();
        assert!((chunker.duration_secs() - 65.0).abs() < 0.01);

        let mut chunks = Vec::new();
        while let Some(chunk) = chunker.next_chunk().unwrap() {
            assert!(chunk.samples.len() <= 33 * 16_000);
            chunks.push(chunk);
        }

        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].start_sample, 0);
        assert_eq!(chunks.last().unwrap().end_sample, 65 * 16_000);
        for pair in chunks.windows(2) {
            assert_eq!(pair[0].end_sample, pair[1].start_sample);
        }
        for chunk in &chunks[..chunks.len() - 1] {
            let secs = (chunk.end_sample - chunk.start_sample) as f32 / 16_000.0;
            assert!((27.0..=33.0).contains(&secs), "bad chunk duration {secs}");
        }
    }

    #[test]
    fn stream_prefers_quiet_split_point() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("quiet.wav");
        write_test_wav(&path, 40, |index| {
            let secs = index as f32 / 16_000.0;
            if (29.0..29.3).contains(&secs) {
                1
            } else {
                7000
            }
        });

        let mut chunker = WavChunker::open(&path).unwrap();
        let first = chunker.next_chunk().unwrap().unwrap();
        let split_secs = first.end_sample as f32 / 16_000.0;
        assert!((28.8..=30.2).contains(&split_secs), "split at {split_secs}");
    }

    #[test]
    fn stream_returns_short_audio_in_one_chunk() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("short.wav");
        write_test_wav(&path, 9, |index| if index % 2 == 0 { 1000 } else { -1000 });

        let mut chunker = WavChunker::open(&path).unwrap();
        let first = chunker.next_chunk().unwrap().unwrap();
        assert_eq!(first.start_sample, 0);
        assert_eq!(first.end_sample, 9 * 16_000);
        assert!(chunker.next_chunk().unwrap().is_none());
    }
}
