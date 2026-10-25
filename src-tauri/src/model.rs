use std::fs::{self, File};
use std::io::{Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use transcribe_rs::onnx::parakeet::{ParakeetModel, ParakeetParams, TimestampGranularity};
use transcribe_rs::onnx::Quantization;

use crate::logging::log;

pub const MODEL_ARCHIVE_URL: &str = "https://blob.handy.computer/parakeet-v3-int8.tar.gz";
pub const MODEL_ARCHIVE_SHA256: &str =
    "43d37191602727524a7d8c6da0eef11c4ba24320f5b4730f1a2497befc2efa77";
pub const MODEL_DIR_NAME: &str = "parakeet-v3-int8";
pub const EXPECTED_FILES: [&str; 5] = [
    "encoder-model.int8.onnx",
    "decoder_joint-model.int8.onnx",
    "nemo128.onnx",
    "vocab.txt",
    "config.json",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelStage {
    ConsentRequired,
    Downloading,
    Loading,
    Ready,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelStatus {
    pub stage: ModelStage,
    pub progress: f32,
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub error: Option<String>,
    /// Monotonic revision assigned by the manager. The UI uses it to ignore
    /// replies and events that arrive out of order.
    pub revision: u64,
}

impl ModelStatus {
    fn new(stage: ModelStage) -> Self {
        Self {
            stage,
            progress: 0.0,
            downloaded_bytes: None,
            total_bytes: None,
            error: None,
            revision: 0,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            stage: ModelStage::Error,
            progress: 0.0,
            downloaded_bytes: None,
            total_bytes: None,
            error: Some(message.into()),
            revision: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelPaths {
    pub models_dir: PathBuf,
    pub model_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl ModelPaths {
    pub fn new(app_data: PathBuf) -> Self {
        let models_dir = app_data.join("models");
        Self {
            model_dir: models_dir.join(MODEL_DIR_NAME),
            cache_dir: app_data.join("cache"),
            models_dir,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum StartAction {
    LoadInstalled,
    Download,
    WaitForConsent,
    AlreadyBusy,
}

impl StartAction {
    pub fn for_state(installed: bool, consent: bool) -> Self {
        if installed {
            Self::LoadInstalled
        } else if consent {
            Self::Download
        } else {
            Self::WaitForConsent
        }
    }
}

pub fn initial_stage(paths: &ModelPaths) -> ModelStage {
    if is_model_installed(&paths.model_dir) {
        ModelStage::Loading
    } else {
        ModelStage::ConsentRequired
    }
}

pub fn is_model_installed(model_dir: &Path) -> bool {
    EXPECTED_FILES
        .iter()
        .all(|name| model_dir.join(name).is_file())
}

pub fn sha256_file(path: &Path) -> Result<String, ModelError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("ошибка файловой системы: {0}")]
    Io(#[from] std::io::Error),
    #[error("ошибка сети: {0}")]
    Http(#[from] reqwest::Error),
    #[error("архив модели не прошёл проверку целостности")]
    Checksum,
    #[error("архив модели не содержит полный набор файлов Parakeet V3")]
    IncompleteArchive,
    #[error("модель ещё не загружена")]
    NotReady,
    #[error("загрузка модели не разрешена")]
    ConsentRequired,
    #[error("не удалось загрузить модель: {0}")]
    Load(String),
}

pub type StatusSink = Arc<dyn Fn(ModelStatus) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineResult {
    pub text: String,
    pub segments: Vec<EngineSegment>,
}

pub trait SpeechEngine: Send {
    fn transcribe(&mut self, samples: &[f32]) -> Result<EngineResult, String>;
}

pub struct ParakeetEngine {
    model: ParakeetModel,
}

impl ParakeetEngine {
    fn load(model_dir: &Path) -> Result<Self, String> {
        let model = ParakeetModel::load(model_dir, &Quantization::Int8)
            .map_err(|error| error.to_string())?;
        Ok(Self { model })
    }
}

impl SpeechEngine for ParakeetEngine {
    fn transcribe(&mut self, samples: &[f32]) -> Result<EngineResult, String> {
        let result = self
            .model
            .transcribe_with(
                samples,
                &ParakeetParams {
                    timestamp_granularity: Some(TimestampGranularity::Segment),
                    ..Default::default()
                },
            )
            .map_err(|error| error.to_string())?;

        Ok(EngineResult {
            text: result.text,
            segments: result
                .segments
                .unwrap_or_default()
                .into_iter()
                .map(|segment| EngineSegment {
                    start: segment.start as f64,
                    end: segment.end as f64,
                    text: segment.text,
                })
                .collect(),
        })
    }
}

type EngineLoader = Arc<dyn Fn(&Path) -> Result<Box<dyn SpeechEngine>, String> + Send + Sync>;

pub struct ModelManager {
    paths: ModelPaths,
    status: Mutex<ModelStatus>,
    engine: Mutex<Option<Box<dyn SpeechEngine>>>,
    sink: StatusSink,
    consent: AtomicBool,
    busy: AtomicBool,
    revision: AtomicU64,
    loader: EngineLoader,
}

impl ModelManager {
    pub fn new(paths: ModelPaths, sink: StatusSink) -> Arc<Self> {
        Self::with_loader(
            paths,
            sink,
            Arc::new(|model_dir| {
                ParakeetEngine::load(model_dir)
                    .map(|engine| Box::new(engine) as Box<dyn SpeechEngine>)
            }),
        )
    }

    fn with_loader(paths: ModelPaths, sink: StatusSink, loader: EngineLoader) -> Arc<Self> {
        let status = ModelStatus::new(initial_stage(&paths));
        Arc::new(Self {
            paths,
            status: Mutex::new(status),
            engine: Mutex::new(None),
            sink,
            consent: AtomicBool::new(false),
            busy: AtomicBool::new(false),
            revision: AtomicU64::new(0),
            loader,
        })
    }

    pub fn status(&self) -> ModelStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| ModelStatus::error("Состояние модели недоступно"))
    }

    pub fn is_ready(&self) -> bool {
        self.status().stage == ModelStage::Ready
    }

    pub fn start_on_launch(self: &Arc<Self>, consent: bool) {
        self.consent.store(consent, Ordering::SeqCst);
        if self.busy.load(Ordering::SeqCst) {
            return;
        }
        let installed = is_model_installed(&self.paths.model_dir);
        match StartAction::for_state(installed, consent) {
            StartAction::LoadInstalled => {
                log("model: found on disk, loading at startup");
                self.spawn_load();
            }
            StartAction::Download => {
                log("model: consent already stored, resuming download at startup");
                self.spawn_download();
            }
            StartAction::WaitForConsent => log("model: consent required before download"),
            StartAction::AlreadyBusy => {}
        }
    }

    pub fn grant_and_download(self: &Arc<Self>) -> Result<(), ModelError> {
        self.consent.store(true, Ordering::SeqCst);
        log("model: consent granted by user");
        if self.busy.load(Ordering::SeqCst) {
            log("model: download already in progress");
            return Ok(());
        }
        if is_model_installed(&self.paths.model_dir) {
            self.spawn_load();
        } else {
            self.spawn_download();
        }
        Ok(())
    }

    pub fn retry(self: &Arc<Self>) -> Result<(), ModelError> {
        if self.busy.load(Ordering::SeqCst) {
            return Ok(());
        }
        if is_model_installed(&self.paths.model_dir) {
            self.spawn_load();
            return Ok(());
        }
        if !self.consent.load(Ordering::SeqCst) {
            return Err(ModelError::ConsentRequired);
        }
        self.spawn_download();
        Ok(())
    }

    pub fn transcribe(&self, samples: &[f32]) -> Result<EngineResult, ModelError> {
        let mut guard = self
            .engine
            .lock()
            .map_err(|_| ModelError::Load("движок недоступен".into()))?;
        let engine = guard.as_mut().ok_or(ModelError::NotReady)?;
        engine.transcribe(samples).map_err(ModelError::Load)
    }

    fn spawn_load(self: &Arc<Self>) {
        if self.busy.swap(true, Ordering::SeqCst) {
            return;
        }
        self.set_status(ModelStatus::new(ModelStage::Loading));
        let manager = Arc::clone(self);
        std::thread::spawn(move || {
            match run_guarded(|| (manager.loader)(&manager.paths.model_dir)) {
                Ok(Ok(engine)) => {
                    match manager.engine.lock() {
                        Ok(mut slot) => *slot = Some(engine),
                        Err(_) => {
                            manager.finish_error("Не удалось сохранить движок".to_string());
                            return;
                        }
                    }
                    let mut status = ModelStatus::new(ModelStage::Ready);
                    status.progress = 100.0;
                    manager.set_status(status);
                    manager.busy.store(false, Ordering::SeqCst);
                }
                Ok(Err(error)) => {
                    manager.finish_error(format!("Не удалось запустить Parakeet V3: {error}"));
                }
                Err(panic) => {
                    manager.finish_error(format!("Внутренняя ошибка запуска модели: {panic}"));
                }
            }
        });
    }

    /// Marks the start of a download synchronously, before the worker thread
    /// touches the network. The UI must never look inert while a request is
    /// being established.
    fn begin_download(&self) -> bool {
        if self.busy.swap(true, Ordering::SeqCst) {
            return false;
        }
        self.set_status(ModelStatus {
            stage: ModelStage::Downloading,
            progress: 0.0,
            downloaded_bytes: Some(0),
            total_bytes: None,
            error: None,
            revision: 0,
        });
        true
    }

    fn spawn_download(self: &Arc<Self>) {
        if !self.begin_download() {
            return;
        }
        let manager = Arc::clone(self);
        std::thread::spawn(
            move || match run_guarded(|| manager.download_and_install()) {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    log(format!("download failed: {error}"));
                    let message = if matches!(&error, ModelError::Http(_)) {
                        "Не удалось связаться с сервером модели. \
Проверьте интернет, прокси и брандмауэр."
                            .to_string()
                    } else {
                        error.to_string()
                    };
                    manager.finish_error(message);
                }
                Err(panic) => {
                    manager.finish_error(format!("Внутренняя ошибка загрузки модели: {panic}"));
                }
            },
        );
    }

    fn download_and_install(&self) -> Result<(), ModelError> {
        fs::create_dir_all(&self.paths.cache_dir)?;
        fs::create_dir_all(&self.paths.models_dir)?;
        let temp = tempfile::Builder::new()
            .prefix("model-download-")
            .tempdir_in(&self.paths.cache_dir)?;
        let archive_path = temp.path().join("parakeet-v3-int8.tar.gz");

        let url = model_archive_url();
        log(format!("download: GET {url}"));
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(60 * 60))
            .user_agent(concat!("Transaudio/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let mut response = client.get(url).send()?.error_for_status()?;
        let total = response.content_length();
        log(format!(
            "download: HTTP {} content-length={total:?}",
            response.status()
        ));

        self.set_status(ModelStatus {
            stage: ModelStage::Downloading,
            progress: 0.0,
            downloaded_bytes: Some(0),
            total_bytes: total,
            error: None,
            revision: 0,
        });

        let mut file = File::create(&archive_path)?;
        let mut downloaded = 0_u64;
        let mut buffer = [0_u8; 128 * 1024];
        let mut last_emit = Instant::now();
        loop {
            let read = response.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])?;
            downloaded += read as u64;
            if last_emit.elapsed() >= Duration::from_millis(120) {
                let progress = total
                    .map(|total| (downloaded as f32 / total as f32) * 100.0)
                    .unwrap_or(0.0);
                self.set_status(ModelStatus {
                    stage: ModelStage::Downloading,
                    progress,
                    downloaded_bytes: Some(downloaded),
                    total_bytes: total,
                    error: None,
                    revision: 0,
                });
                last_emit = Instant::now();
            }
        }
        file.flush()?;
        drop(file);

        let digest = sha256_file(&archive_path)?;
        if digest != MODEL_ARCHIVE_SHA256 {
            log(format!("download: checksum mismatch ({digest})"));
            return Err(ModelError::Checksum);
        }
        log("download: checksum verified");

        self.set_status(ModelStatus {
            stage: ModelStage::Loading,
            progress: 100.0,
            downloaded_bytes: Some(downloaded),
            total_bytes: total,
            error: None,
            revision: 0,
        });

        install_archive(&archive_path, &self.paths.models_dir, |_, _| {})?;
        log("download: model extracted");
        self.load_after_download()
    }

    fn load_after_download(&self) -> Result<(), ModelError> {
        let engine = (self.loader)(&self.paths.model_dir).map_err(ModelError::Load)?;
        match self.engine.lock() {
            Ok(mut slot) => *slot = Some(engine),
            Err(_) => return Err(ModelError::Load("движок недоступен".into())),
        }
        log("model: engine loaded after download");
        let mut status = ModelStatus::new(ModelStage::Ready);
        status.progress = 100.0;
        self.set_status(status);
        self.busy.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn finish_error(&self, message: String) {
        log(format!("model: error: {message}"));
        if let Ok(mut slot) = self.engine.lock() {
            slot.take();
        }
        self.set_status(ModelStatus::error(message));
        self.busy.store(false, Ordering::SeqCst);
    }

    fn set_status(&self, mut status: ModelStatus) {
        status.revision = self.revision.fetch_add(1, Ordering::SeqCst) + 1;
        let changed = self
            .status
            .lock()
            .map(|current| current.stage != status.stage)
            .unwrap_or(true);
        if changed {
            match &status.error {
                Some(error) => log(format!("model: {:?} ({error})", status.stage)),
                None => log(format!("model: {:?}", status.stage)),
            }
        }
        if let Ok(mut current) = self.status.lock() {
            *current = status.clone();
        }
        (self.sink)(status);
    }
}

/// Development-only override used to exercise the failure path. Release
/// builds always download from the pinned source.
fn model_archive_url() -> String {
    #[cfg(debug_assertions)]
    {
        if let Ok(url) = std::env::var("TRANSAUDIO_MODEL_URL") {
            return url;
        }
    }
    MODEL_ARCHIVE_URL.to_string()
}

fn run_guarded<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => Ok(value),
        Err(payload) => Err(panic_message(payload)),
    }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "неизвестная ошибка".into()
    }
}

pub fn install_archive(
    archive_path: &Path,
    models_dir: &Path,
    on_progress: impl Fn(u64, u64),
) -> Result<PathBuf, ModelError> {
    fs::create_dir_all(models_dir)?;
    let unpack = tempfile::Builder::new()
        .prefix(".model-unpack-")
        .tempdir_in(models_dir)?;

    let file = File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let entries = archive.entries()?;
    let mut count = 0_u64;
    for entry in entries {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();
        if path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(ModelError::IncompleteArchive);
        }
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            continue;
        }
        if entry_type.is_dir() || entry_type.is_file() {
            entry.unpack_in(unpack.path())?;
            count += 1;
            on_progress(count, 0);
        }
    }

    let source = fs::read_dir(unpack.path())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_dir() && is_model_installed(path))
        .ok_or(ModelError::IncompleteArchive)?;

    let destination = models_dir.join(MODEL_DIR_NAME);
    if destination.exists() {
        fs::remove_dir_all(&destination)?;
    }
    fs::rename(source, &destination)?;
    on_progress(count, count);
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tempfile::TempDir;

    use super::{
        initial_stage, install_archive, is_model_installed, run_guarded, sha256_file, ModelError,
        ModelManager, ModelPaths, ModelStage, ModelStatus, StartAction, EXPECTED_FILES,
    };

    fn create_model_archive(path: &Path, complete: bool) {
        let file = fs::File::create(path).unwrap();
        let encoder = GzEncoder::new(file, Compression::fast());
        let mut archive = tar::Builder::new(encoder);
        for (index, name) in EXPECTED_FILES.iter().enumerate() {
            if !complete && index == 0 {
                continue;
            }
            let data = format!("test-{name}").into_bytes();
            let mut header = tar::Header::new_gnu();
            header
                .set_path(format!("parakeet-tdt-0.6b-v3-int8/{name}"))
                .unwrap();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            archive.append(&header, data.as_slice()).unwrap();
        }
        let encoder = archive.into_inner().unwrap();
        encoder.finish().unwrap();
    }

    #[test]
    fn missing_model_needs_consent_first() {
        let temp = TempDir::new().unwrap();
        let paths = ModelPaths::new(temp.path().to_path_buf());
        assert_eq!(initial_stage(&paths), ModelStage::ConsentRequired);
        assert_eq!(
            StartAction::for_state(false, false),
            StartAction::WaitForConsent
        );
    }

    #[test]
    fn existing_model_loads_without_consent() {
        let temp = TempDir::new().unwrap();
        let paths = ModelPaths::new(temp.path().to_path_buf());
        fs::create_dir_all(&paths.model_dir).unwrap();
        for name in EXPECTED_FILES {
            fs::write(paths.model_dir.join(name), b"x").unwrap();
        }

        assert!(is_model_installed(&paths.model_dir));
        assert_eq!(initial_stage(&paths), ModelStage::Loading);
        assert_eq!(
            StartAction::for_state(true, false),
            StartAction::LoadInstalled
        );
        assert_eq!(StartAction::for_state(false, true), StartAction::Download);
    }

    #[test]
    fn incomplete_model_is_not_usable() {
        let temp = TempDir::new().unwrap();
        let paths = ModelPaths::new(temp.path().to_path_buf());
        fs::create_dir_all(&paths.model_dir).unwrap();
        fs::write(paths.model_dir.join(EXPECTED_FILES[0]), b"x").unwrap();

        assert!(!is_model_installed(&paths.model_dir));
    }

    #[test]
    fn extracts_archive_into_models_directory() {
        let temp = TempDir::new().unwrap();
        let archive = temp.path().join("model.tar.gz");
        create_model_archive(&archive, true);
        let models_dir = temp.path().join("models");
        fs::create_dir_all(&models_dir).unwrap();

        let installed = install_archive(&archive, &models_dir, |_, _| {}).unwrap();

        assert_eq!(installed, models_dir.join("parakeet-v3-int8"));
        for name in EXPECTED_FILES {
            assert!(installed.join(name).is_file(), "{name} missing");
        }
    }

    #[test]
    fn rejects_archive_missing_model_files() {
        let temp = TempDir::new().unwrap();
        let archive = temp.path().join("broken.tar.gz");
        create_model_archive(&archive, false);
        let models_dir = temp.path().join("models");
        fs::create_dir_all(&models_dir).unwrap();

        let error = install_archive(&archive, &models_dir, |_, _| {}).unwrap_err();
        assert!(error.to_string().contains("модели"));
    }

    #[test]
    fn computes_sha256_for_download_verification() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("payload.bin");
        let mut output = fs::File::create(&file).unwrap();
        output.write_all(b"parakeet").unwrap();
        drop(output);

        let digest = sha256_file(&file).unwrap();
        assert_eq!(
            digest,
            "ea6f44352cee3ddfc1c6161da4ebf33e99e699e7ad24bacc8484fd2c22e94a6d"
        );
    }

    #[test]
    fn manager_reports_initial_status() {
        let temp = TempDir::new().unwrap();
        let paths = ModelPaths::new(temp.path().to_path_buf());
        let manager = ModelManager::new(paths, std::sync::Arc::new(|_: ModelStatus| {}));

        assert_eq!(manager.status().stage, ModelStage::ConsentRequired);
        assert_eq!(manager.status().progress, 0.0);
    }
    #[test]
    fn begin_download_publishes_progress_before_any_network_work() {
        let temp = TempDir::new().unwrap();
        let paths = ModelPaths::new(temp.path().to_path_buf());
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = {
            let seen = std::sync::Arc::clone(&seen);
            std::sync::Arc::new(move |status: ModelStatus| {
                seen.lock().unwrap().push(status);
            })
        };
        let loader =
            std::sync::Arc::new(|_: &Path| -> Result<Box<dyn super::SpeechEngine>, String> {
                Err("loader must not run".into())
            });
        let manager = ModelManager::with_loader(paths, sink, loader);

        assert!(manager.begin_download());
        let status = manager.status();
        assert_eq!(status.stage, ModelStage::Downloading);
        assert_eq!(status.progress, 0.0);
        assert_eq!(status.downloaded_bytes, Some(0));
        assert!(status.error.is_none());
        assert_eq!(
            seen.lock().unwrap().last().unwrap().stage,
            ModelStage::Downloading
        );

        assert!(!manager.begin_download(), "second attempt must not restart");
    }

    #[test]
    fn guarded_run_turns_panics_into_visible_errors() {
        let result = run_guarded(|| -> Result<(), ModelError> { panic!("boom in worker") });
        let message = result.expect_err("panic must not escape silently");
        assert!(message.contains("boom in worker"));
    }

    #[test]
    fn guarded_run_returns_normal_results() {
        assert_eq!(run_guarded(|| 7).unwrap(), 7);
    }
}
