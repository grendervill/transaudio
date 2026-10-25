use std::fs::{self, File};
use std::io::{Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use transcribe_cpp::{Model, RunOptions, Session, TimestampKind};

use crate::catalog::{self, ModelSpec};
use crate::logging::log;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelStage {
    ConsentRequired,
    NotInstalled,
    Downloading,
    Loading,
    Ready,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelStatus {
    pub model_id: String,
    pub stage: ModelStage,
    pub progress: f32,
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub error: Option<String>,
    /// Monotonic revision assigned by the manager. The UI keeps the newest one.
    pub revision: u64,
}

impl ModelStatus {
    fn new(model_id: &str, stage: ModelStage) -> Self {
        Self {
            model_id: model_id.to_string(),
            stage,
            progress: 0.0,
            downloaded_bytes: None,
            total_bytes: None,
            error: None,
            revision: 0,
        }
    }

    fn error(model_id: &str, message: impl Into<String>) -> Self {
        Self {
            model_id: model_id.to_string(),
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
    pub cache_dir: PathBuf,
}

impl ModelPaths {
    pub fn new(app_data: PathBuf) -> Self {
        Self {
            models_dir: app_data.join("models"),
            cache_dir: app_data.join("cache"),
        }
    }

    pub fn dir_for(&self, spec: &ModelSpec) -> PathBuf {
        self.models_dir.join(spec.id)
    }

    pub fn file_for(&self, spec: &ModelSpec) -> PathBuf {
        self.dir_for(spec).join(spec.filename)
    }
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("ошибка файловой системы: {0}")]
    Io(#[from] std::io::Error),
    #[error("ошибка сети: {0}")]
    Http(#[from] reqwest::Error),
    #[error("файл модели не прошёл проверку целостности")]
    Checksum,
    #[error("неизвестная модель: {0}")]
    UnknownModel(String),
    #[error("модель ещё не загружена")]
    NotReady,
    #[error("загрузка модели не разрешена")]
    ConsentRequired,
    #[error("дождитесь завершения текущей операции с моделью")]
    Busy,
    #[error("нельзя удалить выбранную модель")]
    DeleteSelected,
    #[error("не удалось загрузить модель: {0}")]
    Load(String),
}

pub type StatusSink = Arc<dyn Fn(ModelStatus) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineResult {
    pub text: String,
    pub segments: Vec<EngineSegment>,
}

pub trait SpeechEngine: Send {
    fn transcribe(&mut self, samples: &[f32]) -> Result<EngineResult, String>;
}

pub struct CppEngine {
    session: Session,
    _model: Model,
    language: Arc<Mutex<Option<String>>>,
}

impl SpeechEngine for CppEngine {
    fn transcribe(&mut self, samples: &[f32]) -> Result<EngineResult, String> {
        let language = self
            .language
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
            .filter(|value| !value.is_empty());
        let options = RunOptions {
            language,
            timestamps: TimestampKind::Segment,
            ..Default::default()
        };
        let transcript = self
            .session
            .run(samples, &options)
            .map_err(|error| error.to_string())?;
        let segments = transcript
            .segments
            .into_iter()
            .map(|segment| EngineSegment {
                start: segment.t0_ms as f64 / 1000.0,
                end: segment.t1_ms as f64 / 1000.0,
                text: segment.text,
                speaker: None,
            })
            .collect();
        Ok(EngineResult {
            text: transcript.text,
            segments,
        })
    }
}

type LanguageSlot = Arc<Mutex<Option<String>>>;
type EngineLoader =
    Arc<dyn Fn(&Path, LanguageSlot) -> Result<Box<dyn SpeechEngine>, String> + Send + Sync>;

fn load_cpp_engine(path: &Path, language: LanguageSlot) -> Result<Box<dyn SpeechEngine>, String> {
    let model = Model::load(path).map_err(|error| error.to_string())?;
    let session = model.session().map_err(|error| error.to_string())?;
    Ok(Box::new(CppEngine {
        session,
        _model: model,
        language,
    }))
}

pub struct ModelManager {
    paths: ModelPaths,
    status: Mutex<ModelStatus>,
    engine: Mutex<Option<Box<dyn SpeechEngine>>>,
    sink: StatusSink,
    consent: AtomicBool,
    busy: AtomicBool,
    revision: AtomicU64,
    selected: Mutex<String>,
    language: LanguageSlot,
    loader: EngineLoader,
}

impl ModelManager {
    pub fn new(paths: ModelPaths, sink: StatusSink) -> Arc<Self> {
        Self::with_loader(paths, sink, Arc::new(load_cpp_engine))
    }

    fn with_loader(paths: ModelPaths, sink: StatusSink, loader: EngineLoader) -> Arc<Self> {
        let selected = catalog::default_model().id.to_string();
        Arc::new(Self {
            status: Mutex::new(ModelStatus::new(&selected, ModelStage::ConsentRequired)),
            paths,
            engine: Mutex::new(None),
            sink,
            consent: AtomicBool::new(false),
            busy: AtomicBool::new(false),
            revision: AtomicU64::new(0),
            selected: Mutex::new(selected),
            language: Arc::new(Mutex::new(None)),
            loader,
        })
    }

    pub fn selected_id(&self) -> String {
        self.selected
            .lock()
            .map(|value| value.clone())
            .unwrap_or_else(|_| catalog::default_model().id.to_string())
    }

    pub fn status(&self) -> ModelStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| {
                ModelStatus::error(&self.selected_id(), "Состояние модели недоступно")
            })
    }

    pub fn selected_name(&self) -> String {
        catalog::find(&self.selected_id())
            .map(|spec| spec.name.to_string())
            .unwrap_or_else(|| "Transaudio".into())
    }

    pub fn is_ready(&self) -> bool {
        self.status().stage == ModelStage::Ready
    }

    pub fn start_on_launch(
        self: &Arc<Self>,
        consent: bool,
        selected: &str,
        language: Option<String>,
    ) {
        self.consent.store(consent, Ordering::SeqCst);
        *self
            .selected
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = selected.to_string();
        *self
            .language
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = language;

        let Some(spec) = catalog::find(selected) else {
            self.set_status(ModelStatus::error(selected, "Модель не найдена"));
            return;
        };
        log(format!(
            "model: startup selected={} consent={consent}",
            spec.id
        ));
        if is_installed(spec, &self.paths.models_dir) {
            self.spawn_load(*spec);
        } else if consent {
            self.set_status(ModelStatus::new(spec.id, ModelStage::NotInstalled));
        } else {
            self.set_status(ModelStatus::new(spec.id, ModelStage::ConsentRequired));
            log("model: consent required before download");
        }
    }

    pub fn select(
        self: &Arc<Self>,
        id: &str,
        language: Option<String>,
    ) -> Result<ModelStatus, ModelError> {
        let spec = catalog::find(id).ok_or_else(|| ModelError::UnknownModel(id.to_string()))?;
        self.select_spec(spec, language)
    }

    fn select_spec(
        self: &Arc<Self>,
        spec: &ModelSpec,
        language: Option<String>,
    ) -> Result<ModelStatus, ModelError> {
        if self.busy.load(Ordering::SeqCst) {
            return Err(ModelError::Busy);
        }
        *self
            .selected
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = spec.id.to_string();
        *self
            .language
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = language;
        if let Ok(mut engine) = self.engine.lock() {
            engine.take();
        }
        if is_installed(spec, &self.paths.models_dir) {
            self.spawn_load(*spec);
        } else {
            self.set_status(ModelStatus::new(spec.id, ModelStage::NotInstalled));
        }
        Ok(self.status())
    }

    pub fn download(
        self: &Arc<Self>,
        id: &str,
        language: Option<String>,
    ) -> Result<ModelStatus, ModelError> {
        let spec = catalog::find(id).ok_or_else(|| ModelError::UnknownModel(id.to_string()))?;
        if self.busy.load(Ordering::SeqCst) {
            return Err(ModelError::Busy);
        }
        if !self.consent.load(Ordering::SeqCst) {
            return Err(ModelError::ConsentRequired);
        }
        *self
            .selected
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = spec.id.to_string();
        *self
            .language
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = language;

        if is_installed(spec, &self.paths.models_dir) {
            if let Ok(mut engine) = self.engine.lock() {
                engine.take();
            }
            self.spawn_load(*spec);
            return Ok(self.status());
        }
        self.spawn_download(*spec);
        Ok(self.status())
    }

    pub fn delete(&self, id: &str) -> Result<(), ModelError> {
        if self.busy.load(Ordering::SeqCst) {
            return Err(ModelError::Busy);
        }
        if self.selected_id() == id {
            return Err(ModelError::DeleteSelected);
        }
        let spec = catalog::find(id).ok_or_else(|| ModelError::UnknownModel(id.to_string()))?;
        let dir = self.paths.dir_for(spec);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
            log(format!("model: deleted {}", spec.id));
        }
        Ok(())
    }

    pub fn set_language(&self, language: Option<String>) {
        *self
            .language
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = language;
    }

    pub fn transcribe(&self, samples: &[f32]) -> Result<EngineResult, ModelError> {
        let mut guard = self
            .engine
            .lock()
            .map_err(|_| ModelError::Load("движок недоступен".into()))?;
        let engine = guard.as_mut().ok_or(ModelError::NotReady)?;
        engine.transcribe(samples).map_err(ModelError::Load)
    }

    fn spawn_load(self: &Arc<Self>, spec: ModelSpec) {
        if self.busy.swap(true, Ordering::SeqCst) {
            return;
        }
        self.set_status(ModelStatus::new(spec.id, ModelStage::Loading));
        let manager = Arc::clone(self);
        let path = self.paths.file_for(&spec);
        let language = Arc::clone(&self.language);
        std::thread::spawn(
            move || match run_guarded(|| (manager.loader)(&path, language)) {
                Ok(Ok(engine)) => match manager.engine.lock() {
                    Ok(mut slot) => {
                        *slot = Some(engine);
                        log(format!("model: {} loaded", spec.id));
                        let mut status = ModelStatus::new(spec.id, ModelStage::Ready);
                        status.progress = 100.0;
                        manager.set_status(status);
                        manager.busy.store(false, Ordering::SeqCst);
                    }
                    Err(_) => manager.finish_error(spec.id, "Не удалось сохранить движок"),
                },
                Ok(Err(error)) => {
                    manager.finish_error(spec.id, &format!("Не удалось запустить модель: {error}"))
                }
                Err(panic) => manager.finish_error(
                    spec.id,
                    &format!("Внутренняя ошибка запуска модели: {panic}"),
                ),
            },
        );
    }

    fn spawn_download(self: &Arc<Self>, spec: ModelSpec) {
        if !self.begin_download(&spec) {
            return;
        }
        let manager = Arc::clone(self);
        std::thread::spawn(
            move || match run_guarded(|| manager.download_and_load(spec)) {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    log(format!("model download failed: {error}"));
                    let message = match &error {
                        ModelError::Http(_) => "Не удалось связаться с сервером модели. \
Проверьте интернет, прокси и брандмауэр."
                            .to_string(),
                        _ => error.to_string(),
                    };
                    manager.finish_error(spec.id, &message);
                }
                Err(panic) => manager.finish_error(
                    spec.id,
                    &format!("Внутренняя ошибка загрузки модели: {panic}"),
                ),
            },
        );
    }

    fn begin_download(&self, spec: &ModelSpec) -> bool {
        if self.busy.swap(true, Ordering::SeqCst) {
            return false;
        }
        self.set_status(ModelStatus {
            model_id: spec.id.to_string(),
            stage: ModelStage::Downloading,
            progress: 0.0,
            downloaded_bytes: Some(0),
            total_bytes: Some(spec.size_bytes),
            error: None,
            revision: 0,
        });
        true
    }

    fn download_and_load(&self, spec: ModelSpec) -> Result<(), ModelError> {
        let url = catalog::download_url(&spec);
        fs::create_dir_all(&self.paths.cache_dir)?;
        let part = self.paths.cache_dir.join(format!("{}.part", spec.id));
        if part.exists() {
            fs::remove_file(&part)?;
        }
        log(format!("download: {} -> {}", spec.id, url));

        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(60 * 120))
            .user_agent(concat!("Transaudio/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let mut response = client.get(url).send()?.error_for_status()?;
        let total = response.content_length().unwrap_or(spec.size_bytes);

        let mut file = File::create(&part)?;
        let mut downloaded = 0_u64;
        let mut buffer = [0_u8; 256 * 1024];
        let mut last_emit = Instant::now();
        loop {
            let read = response.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])?;
            downloaded += read as u64;
            if last_emit.elapsed() >= Duration::from_millis(120) {
                let progress = (downloaded as f32 / total.max(1) as f32 * 100.0).min(99.5);
                self.set_status(ModelStatus {
                    model_id: spec.id.to_string(),
                    stage: ModelStage::Downloading,
                    progress,
                    downloaded_bytes: Some(downloaded),
                    total_bytes: Some(total),
                    error: None,
                    revision: 0,
                });
                last_emit = Instant::now();
            }
        }
        file.flush()?;
        drop(file);

        if let Err(error) = install_model_file(&spec, &part, &self.paths.models_dir) {
            let _ = fs::remove_file(&part);
            return Err(error);
        }
        if spec.id == "parakeet-tdt-0.6b-v3" {
            let legacy = self.paths.models_dir.join("parakeet-v3-int8");
            if legacy.exists() {
                let _ = fs::remove_dir_all(&legacy);
                log("model: removed legacy ONNX parakeet");
            }
        }

        self.set_status(ModelStatus {
            model_id: spec.id.to_string(),
            stage: ModelStage::Loading,
            progress: 100.0,
            downloaded_bytes: Some(downloaded),
            total_bytes: Some(total),
            error: None,
            revision: 0,
        });
        let path = self.paths.file_for(&spec);
        let engine = (self.loader)(&path, Arc::clone(&self.language)).map_err(ModelError::Load)?;
        match self.engine.lock() {
            Ok(mut slot) => *slot = Some(engine),
            Err(_) => return Err(ModelError::Load("движок недоступен".into())),
        }
        let mut status = ModelStatus::new(spec.id, ModelStage::Ready);
        status.progress = 100.0;
        self.set_status(status);
        self.busy.store(false, Ordering::SeqCst);
        log(format!("model: {} ready", spec.id));
        Ok(())
    }

    fn finish_error(&self, model_id: &str, message: &str) {
        log(format!("model: error: {message}"));
        if let Ok(mut slot) = self.engine.lock() {
            slot.take();
        }
        self.set_status(ModelStatus::error(model_id, message));
        self.busy.store(false, Ordering::SeqCst);
    }

    fn set_status(&self, mut status: ModelStatus) {
        status.revision = self.revision.fetch_add(1, Ordering::SeqCst) + 1;
        let changed = self
            .status
            .lock()
            .map(|current| current.stage != status.stage || current.model_id != status.model_id)
            .unwrap_or(true);
        if changed {
            match &status.error {
                Some(error) => log(format!(
                    "model: {} {:?} ({error})",
                    status.model_id, status.stage
                )),
                None => log(format!("model: {} {:?}", status.model_id, status.stage)),
            }
        }
        if let Ok(mut current) = self.status.lock() {
            *current = status.clone();
        }
        (self.sink)(status);
    }
}

pub fn is_installed(spec: &ModelSpec, models_dir: &Path) -> bool {
    let path = models_dir.join(spec.id).join(spec.filename);
    fs::metadata(&path)
        .map(|meta| meta.len() == spec.size_bytes)
        .unwrap_or(false)
}

pub fn sha256_file(path: &Path) -> Result<String, ModelError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 256 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn install_model_file(
    spec: &ModelSpec,
    source: &Path,
    models_dir: &Path,
) -> Result<PathBuf, ModelError> {
    let digest = sha256_file(source)?;
    if digest != spec.sha256 {
        return Err(ModelError::Checksum);
    }
    let dir = models_dir.join(spec.id);
    fs::create_dir_all(&dir)?;
    let destination = dir.join(spec.filename);
    if destination.exists() {
        fs::remove_file(&destination)?;
    }
    fs::rename(source, &destination)?;
    Ok(destination)
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use tempfile::TempDir;

    use super::*;
    use crate::catalog::ModelFamily;

    const TEST_DATA: &[u8] = b"test";
    const TEST_SHA: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

    fn test_spec() -> ModelSpec {
        ModelSpec {
            id: "test-model",
            name: "Test",
            description: "Test",
            family: ModelFamily::Parakeet,
            languages: &["en"],
            language_count: 1,
            quality_en: Some(1.0),
            quality_ru: None,
            speed_x: 1.0,
            size_bytes: TEST_DATA.len() as u64,
            filename: "test.gguf",
            repository: "test/repo",
            revision: "rev",
            sha256: TEST_SHA,
            license: "MIT",
            needs_language: false,
            recommended_for: &[],
            badge: None,
        }
    }

    struct FakeEngine;
    impl SpeechEngine for FakeEngine {
        fn transcribe(&mut self, _samples: &[f32]) -> Result<EngineResult, String> {
            Ok(EngineResult {
                text: "ok".into(),
                segments: Vec::new(),
            })
        }
    }

    fn fake_loader_calls() -> (EngineLoader, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let loader: EngineLoader = Arc::new(move |_path: &Path, _language: LanguageSlot| {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(FakeEngine) as Box<dyn SpeechEngine>)
        });
        (loader, calls)
    }

    #[test]
    fn install_rejects_a_corrupted_file() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("download.part");
        fs::write(&source, b"nope").unwrap();

        let error = install_model_file(&test_spec(), &source, temp.path()).unwrap_err();
        assert!(matches!(error, ModelError::Checksum));
        assert!(!temp.path().join("test-model/test.gguf").exists());
    }

    #[test]
    fn install_moves_a_verified_file_into_place() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("download.part");
        fs::write(&source, TEST_DATA).unwrap();

        let installed = install_model_file(&test_spec(), &source, temp.path()).unwrap();

        assert_eq!(installed, temp.path().join("test-model/test.gguf"));
        assert_eq!(fs::read(&installed).unwrap(), TEST_DATA);
        assert!(!source.exists());
        assert!(is_installed(&test_spec(), temp.path()));
    }

    #[test]
    fn installed_requires_the_exact_size() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("test-model");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("test.gguf");
        fs::write(&file, b"tes").unwrap();
        assert!(!is_installed(&test_spec(), temp.path()));

        fs::write(&file, TEST_DATA).unwrap();
        assert!(is_installed(&test_spec(), temp.path()));
    }

    #[test]
    fn begin_download_reports_progress_before_any_network_work() {
        let temp = TempDir::new().unwrap();
        let paths = ModelPaths::new(temp.path().to_path_buf());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = {
            let seen = Arc::clone(&seen);
            Arc::new(move |status: ModelStatus| {
                seen.lock().unwrap().push(status);
            })
        };
        let (loader, _) = fake_loader_calls();
        let manager = ModelManager::with_loader(paths, sink, loader);

        assert!(manager.begin_download(&test_spec()));
        let status = manager.status();
        assert_eq!(status.stage, ModelStage::Downloading);
        assert_eq!(status.model_id, "test-model");
        assert_eq!(status.progress, 0.0);
        assert_eq!(status.total_bytes, Some(TEST_DATA.len() as u64));
        assert!(status.revision > 0);
        assert_eq!(
            seen.lock().unwrap().last().unwrap().stage,
            ModelStage::Downloading
        );

        assert!(!manager.begin_download(&test_spec()));
    }

    #[test]
    fn selecting_a_missing_model_reports_not_installed() {
        let temp = TempDir::new().unwrap();
        let paths = ModelPaths::new(temp.path().to_path_buf());
        let (loader, calls) = fake_loader_calls();
        let manager = ModelManager::with_loader(paths, Arc::new(|_| {}), loader);

        let status = manager
            .select_spec(&test_spec(), Some("ru".into()))
            .unwrap();

        assert_eq!(status.stage, ModelStage::NotInstalled);
        assert_eq!(manager.selected_id(), "test-model");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(*manager.language.lock().unwrap(), Some("ru".to_string()));
    }

    #[test]
    fn selecting_an_installed_model_loads_it() {
        let temp = TempDir::new().unwrap();
        let paths = ModelPaths::new(temp.path().to_path_buf());
        let spec = test_spec();
        let dir = paths.dir_for(&spec);
        fs::create_dir_all(&dir).unwrap();
        let mut file = fs::File::create(dir.join(spec.filename)).unwrap();
        file.write_all(TEST_DATA).unwrap();
        drop(file);

        let (loader, calls) = fake_loader_calls();
        let manager = ModelManager::with_loader(paths, Arc::new(|_| {}), loader);
        let first = manager.select_spec(&spec, None).unwrap();
        assert!(matches!(
            first.stage,
            ModelStage::Loading | ModelStage::Ready
        ));

        for _ in 0..100 {
            if manager.status().stage == ModelStage::Ready {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(manager.status().stage, ModelStage::Ready);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(manager.transcribe(&[0.0]).unwrap().text, "ok");
    }

    #[test]
    fn cannot_delete_the_selected_model() {
        let temp = TempDir::new().unwrap();
        let paths = ModelPaths::new(temp.path().to_path_buf());
        let (loader, _) = fake_loader_calls();
        let manager = ModelManager::with_loader(paths, Arc::new(|_| {}), loader);
        manager.select_spec(&test_spec(), None).unwrap();

        let error = manager.delete("test-model").unwrap_err();
        assert!(matches!(error, ModelError::DeleteSelected));
    }

    #[test]
    fn guarded_run_turns_panics_into_visible_errors() {
        let result = run_guarded(|| -> Result<(), ModelError> { panic!("boom in worker") });
        let message = result.expect_err("panic must not escape silently");
        assert!(message.contains("boom in worker"));
    }
}
