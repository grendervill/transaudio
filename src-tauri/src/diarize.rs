use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sherpa_onnx::{
    FastClusteringConfig, OfflineSpeakerDiarization, OfflineSpeakerDiarizationConfig,
    OfflineSpeakerSegmentationModelConfig, OfflineSpeakerSegmentationPyannoteModelConfig,
    SpeakerEmbeddingExtractorConfig,
};

use crate::logging::log;

pub const SEGMENTATION_URL: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-pyannote-segmentation-3-0/resolve/main/model.onnx";
pub const SEGMENTATION_SHA256: &str =
    "220ad67ca923bef2fa91f2390c786097bf305bceb5e261d4af67b38e938e1079";
pub const SEGMENTATION_SIZE: u64 = 5_992_913;

pub const EMBEDDING_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx";
pub const EMBEDDING_SHA256: &str =
    "f682b514c05d947ee3fa91cd6ec6c5c7543479a128373fa29b1faedccd21fd11";
pub const EMBEDDING_SIZE: u64 = 28_281_138;

const MAX_GAP_SECS: f64 = 0.4;
const MIN_TURN_SECS: f64 = 0.5;
const PADDING_SECS: f64 = 0.15;
const MAX_TURN_SECS: f64 = 30.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiarizationStage {
    NotDownloaded,
    Downloading,
    Ready,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiarizationStatus {
    pub stage: DiarizationStage,
    pub progress: f32,
    pub error: Option<String>,
    pub revision: u64,
}

impl Default for DiarizationStatus {
    fn default() -> Self {
        Self {
            stage: DiarizationStage::NotDownloaded,
            progress: 0.0,
            error: None,
            revision: 0,
        }
    }
}

impl DiarizationStatus {
    fn error(message: impl Into<String>) -> Self {
        Self {
            stage: DiarizationStage::Error,
            progress: 0.0,
            error: Some(message.into()),
            revision: 0,
        }
    }
}

pub type DiarizationSink = Arc<dyn Fn(DiarizationStatus) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Turn {
    pub start: f64,
    pub end: f64,
    pub speaker: u32,
}

pub struct DiarizationManager {
    dir: PathBuf,
    segmentation: PathBuf,
    embedding: PathBuf,
    status: Mutex<DiarizationStatus>,
    sink: DiarizationSink,
    busy: AtomicBool,
    revision: AtomicU64,
}

impl DiarizationManager {
    pub fn new(dir: PathBuf, sink: DiarizationSink) -> Arc<Self> {
        let manager = Arc::new(Self {
            segmentation: dir.join("segmentation.onnx"),
            embedding: dir.join("embedding.onnx"),
            dir,
            status: Mutex::new(DiarizationStatus::default()),
            sink,
            busy: AtomicBool::new(false),
            revision: AtomicU64::new(0),
        });
        if manager.is_ready() {
            manager.set_status(DiarizationStatus {
                stage: DiarizationStage::Ready,
                progress: 100.0,
                error: None,
                revision: 0,
            });
        }
        manager
    }

    pub fn status(&self) -> DiarizationStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| DiarizationStatus::error("Состояние спикеров недоступно"))
    }

    pub fn is_ready(&self) -> bool {
        has_size(&self.segmentation, SEGMENTATION_SIZE) && has_size(&self.embedding, EMBEDDING_SIZE)
    }

    pub fn start_download(self: &Arc<Self>) {
        if self.busy.swap(true, Ordering::SeqCst) {
            return;
        }
        self.set_status(DiarizationStatus {
            stage: DiarizationStage::Downloading,
            progress: 0.0,
            error: None,
            revision: 0,
        });
        let manager = Arc::clone(self);
        std::thread::spawn(move || {
            match manager.ensure_ready_blocking() {
                Ok(()) => manager.set_status(DiarizationStatus {
                    stage: DiarizationStage::Ready,
                    progress: 100.0,
                    error: None,
                    revision: 0,
                }),
                Err(error) => {
                    log(format!("diarization download failed: {error}"));
                    manager.set_status(DiarizationStatus::error(error));
                }
            }
            manager.busy.store(false, Ordering::SeqCst);
        });
    }

    /// Downloads missing models synchronously and verifies their checksums.
    pub fn ensure_ready_blocking(&self) -> Result<(), String> {
        fs::create_dir_all(&self.dir).map_err(|error| error.to_string())?;
        if self.is_ready() {
            return Ok(());
        }
        let cache = self.dir.join(".cache");
        fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(60 * 30))
            .user_agent(concat!("Transaudio/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| error.to_string())?;

        if !has_size(&self.segmentation, SEGMENTATION_SIZE) {
            let part = cache.join("segmentation.part");
            self.set_status(status_downloading(0.0));
            download(
                &client,
                SEGMENTATION_URL,
                &part,
                SEGMENTATION_SIZE,
                |fraction| {
                    self.set_status(status_downloading(fraction * 20.0));
                },
            )?;
            install_verified(&part, SEGMENTATION_SHA256, &self.segmentation)?;
            log("diarization: segmentation installed");
        }

        if !has_size(&self.embedding, EMBEDDING_SIZE) {
            let part = cache.join("embedding.part");
            self.set_status(status_downloading(20.0));
            download(&client, EMBEDDING_URL, &part, EMBEDDING_SIZE, |fraction| {
                self.set_status(status_downloading(20.0 + fraction * 80.0));
            })?;
            install_verified(&part, EMBEDDING_SHA256, &self.embedding)?;
            log("diarization: embedding installed");
        }

        let _ = fs::remove_dir_all(&cache);
        Ok(())
    }

    pub fn diarize(&self, samples: &[f32], scenario: &str) -> Result<Vec<Turn>, String> {
        if !self.is_ready() {
            return Err("Модели спикеров не подготовлены".into());
        }
        let threshold = scenario_threshold(scenario);
        let config = OfflineSpeakerDiarizationConfig {
            segmentation: OfflineSpeakerSegmentationModelConfig {
                pyannote: OfflineSpeakerSegmentationPyannoteModelConfig {
                    model: Some(self.segmentation.display().to_string()),
                    window_shift_ratio: 0.1,
                },
                num_threads: 2,
                debug: false,
                provider: Some("cpu".into()),
            },
            embedding: SpeakerEmbeddingExtractorConfig {
                model: Some(self.embedding.display().to_string()),
                num_threads: 2,
                debug: false,
                provider: Some("cpu".into()),
            },
            clustering: FastClusteringConfig {
                num_clusters: -1,
                threshold,
                compute_confidence: true,
            },
            min_duration_on: 0.3,
            min_duration_off: 0.5,
        };
        let Some(diarizer) = OfflineSpeakerDiarization::create(&config) else {
            return Err("Не удалось загрузить модели спикеров".into());
        };
        let Some(result) = diarizer.process(samples) else {
            return Err("Не удалось разделить запись на спикеров".into());
        };
        let turns: Vec<Turn> = result
            .sort_by_start_time()
            .into_iter()
            .map(|segment| Turn {
                start: segment.start as f64,
                end: segment.end as f64,
                speaker: (segment.speaker + 1).max(1) as u32,
            })
            .collect();
        Ok(post_process(&turns))
    }

    fn set_status(&self, mut status: DiarizationStatus) {
        status.revision = self.revision.fetch_add(1, Ordering::SeqCst) + 1;
        let changed = self
            .status
            .lock()
            .map(|current| current.stage != status.stage)
            .unwrap_or(true);
        if changed {
            match &status.error {
                Some(error) => log(format!("diarization: {:?} ({error})", status.stage)),
                None => log(format!("diarization: {:?}", status.stage)),
            }
        }
        if let Ok(mut current) = self.status.lock() {
            *current = status.clone();
        }
        (self.sink)(status);
    }
}

fn status_downloading(progress: f32) -> DiarizationStatus {
    DiarizationStatus {
        stage: DiarizationStage::Downloading,
        progress,
        error: None,
        revision: 0,
    }
}

fn has_size(path: &Path, size: u64) -> bool {
    fs::metadata(path)
        .map(|meta| meta.len() == size)
        .unwrap_or(false)
}

pub fn scenario_threshold(scenario: &str) -> f32 {
    match scenario {
        "meeting" => 0.35,
        "many" => 0.30,
        "auto" => 0.40,
        _ => 0.45,
    }
}

/// Merges short gaps, absorbs blips, pads boundaries and splits long turns.
/// Speaker ids are renumbered in order of their first appearance.
pub fn post_process(turns: &[Turn]) -> Vec<Turn> {
    let mut sorted = turns.to_vec();
    sorted.sort_by(|left, right| left.start.total_cmp(&right.start));
    sorted.retain(|turn| turn.end > turn.start);

    // Absorb turns shorter than MIN_TURN_SECS into the previous one.
    let mut cleaned: Vec<Turn> = Vec::new();
    for turn in sorted {
        if turn.end - turn.start < MIN_TURN_SECS {
            if let Some(previous) = cleaned.last_mut() {
                previous.end = previous.end.max(turn.end);
                continue;
            }
        }
        cleaned.push(turn);
    }

    // Merge adjacent turns of the same speaker when the gap is small.
    let mut merged: Vec<Turn> = Vec::new();
    for turn in cleaned {
        if let Some(previous) = merged.last_mut() {
            if previous.speaker == turn.speaker && turn.start - previous.end <= MAX_GAP_SECS {
                previous.end = turn.end;
                continue;
            }
        }
        merged.push(turn);
    }

    let mut out = Vec::new();
    let mut previous_padded_end = 0.0_f64;
    for (index, turn) in merged.iter().enumerate() {
        let next_start = merged
            .get(index + 1)
            .map(|next| next.start)
            .unwrap_or(f64::MAX);
        let start = (turn.start - PADDING_SECS)
            .max(previous_padded_end)
            .max(0.0);
        let end = (turn.end + PADDING_SECS).min(next_start);
        previous_padded_end = end.max(start);
        let mut cursor = start;
        while cursor < end {
            let chunk_end = (cursor + MAX_TURN_SECS).min(end);
            out.push(Turn {
                start: cursor,
                end: chunk_end,
                speaker: turn.speaker,
            });
            cursor = chunk_end;
        }
    }

    let mut mapping = std::collections::HashMap::new();
    let mut next_id = 1_u32;
    for turn in &mut out {
        let id = *mapping.entry(turn.speaker).or_insert_with(|| {
            let id = next_id;
            next_id += 1;
            id
        });
        turn.speaker = id;
    }
    out
}

fn download(
    client: &reqwest::blocking::Client,
    url: &str,
    destination: &Path,
    expected_size: u64,
    mut on_progress: impl FnMut(f32),
) -> Result<(), String> {
    let mut response = client
        .get(url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("Не удалось скачать модель спикеров: {error}"))?;
    let total = response.content_length().unwrap_or(expected_size).max(1);
    let mut file = File::create(destination).map_err(|error| error.to_string())?;
    let mut downloaded = 0_u64;
    let mut buffer = [0_u8; 256 * 1024];
    let mut last_emit = Instant::now();
    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|error| error.to_string())?;
        downloaded += read as u64;
        if last_emit.elapsed() >= Duration::from_millis(120) {
            on_progress((downloaded as f32 / total as f32).min(1.0));
            last_emit = Instant::now();
        }
    }
    file.flush().map_err(|error| error.to_string())?;
    on_progress(1.0);
    Ok(())
}

fn install_verified(part: &Path, expected: &str, destination: &Path) -> Result<(), String> {
    let digest = sha256(part)?;
    if digest != expected {
        let _ = fs::remove_file(part);
        return Err("Файл модели спикеров не прошёл проверку целостности".into());
    }
    if destination.exists() {
        let _ = fs::remove_file(destination);
    }
    fs::rename(part, destination).map_err(|error| error.to_string())
}

fn sha256(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(start: f64, end: f64, speaker: u32) -> Turn {
        Turn {
            start,
            end,
            speaker,
        }
    }

    #[test]
    fn scenario_thresholds_match_the_spec() {
        assert_eq!(scenario_threshold("interview"), 0.45);
        assert_eq!(scenario_threshold("meeting"), 0.35);
        assert_eq!(scenario_threshold("many"), 0.30);
        assert_eq!(scenario_threshold("auto"), 0.40);
        assert_eq!(scenario_threshold("unknown"), 0.45);
    }

    #[test]
    fn merges_same_speaker_turns_with_short_gaps() {
        let result = post_process(&[turn(0.0, 2.0, 1), turn(2.2, 4.0, 1), turn(4.6, 6.0, 2)]);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].speaker, 1);
        assert!((result[0].end - 4.15).abs() < 1e-6);
        assert_eq!(result[1].speaker, 2);
    }

    #[test]
    fn absorbs_blips_shorter_than_half_a_second() {
        let result = post_process(&[turn(0.0, 2.0, 1), turn(2.1, 2.4, 2), turn(2.5, 4.0, 1)]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].speaker, 1);
        assert!((result[0].end - 4.15).abs() < 1e-6);
    }

    #[test]
    fn pads_boundaries_without_overlapping() {
        let result = post_process(&[turn(1.0, 2.0, 1), turn(2.05, 3.0, 2)]);
        assert!((result[0].start - 0.85).abs() < 1e-6);
        assert!((result[0].end - 2.05).abs() < 1e-6);
        assert!((result[1].start - 2.05).abs() < 1e-6);
    }

    #[test]
    fn splits_turns_longer_than_thirty_seconds() {
        let result = post_process(&[turn(0.0, 65.0, 1)]);
        assert_eq!(result.len(), 3);
        assert!((result[0].end - 30.0).abs() < 1e-6);
        assert!((result[1].start - 30.0).abs() < 1e-6);
        assert!((result[2].end - 65.15).abs() < 1e-6);
    }

    #[test]
    fn renumbers_speakers_by_first_appearance() {
        let result = post_process(&[turn(0.0, 2.0, 7), turn(3.0, 5.0, 3), turn(6.0, 8.0, 7)]);
        assert_eq!(result[0].speaker, 1);
        assert_eq!(result[1].speaker, 2);
        assert_eq!(result[2].speaker, 1);
    }

    #[test]
    fn reports_not_downloaded_until_both_models_exist() {
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path().join("diarization");
        let manager = DiarizationManager::new(dir.clone(), Arc::new(|_| {}));
        assert_eq!(manager.status().stage, DiarizationStage::NotDownloaded);

        fs::create_dir_all(&dir).unwrap();
        File::create(dir.join("segmentation.onnx"))
            .unwrap()
            .set_len(SEGMENTATION_SIZE)
            .unwrap();
        assert_eq!(manager.status().stage, DiarizationStage::NotDownloaded);

        File::create(dir.join("embedding.onnx"))
            .unwrap()
            .set_len(EMBEDDING_SIZE)
            .unwrap();
        let manager = DiarizationManager::new(dir, Arc::new(|_| {}));
        assert_eq!(manager.status().stage, DiarizationStage::Ready);
        assert!(manager.is_ready());
    }
}
