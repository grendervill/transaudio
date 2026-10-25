use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::audio::{AudioEngine, WavChunker, SAMPLE_RATE};
use crate::config::ExportFormat;
use crate::export;
use crate::model::{EngineSegment, ModelManager};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueueStatus {
    Idle,
    Running,
    Paused,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueueState {
    pub status: QueueStatus,
    pub overall_progress: f32,
    pub current_id: Option<String>,
    pub current_index: usize,
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub message: Option<String>,
}

impl Default for QueueState {
    fn default() -> Self {
        Self {
            status: QueueStatus::Idle,
            overall_progress: 0.0,
            current_id: None,
            current_index: 0,
            total: 0,
            completed: 0,
            failed: 0,
            message: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Queued,
    Processing,
    Paused,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileJob {
    pub id: String,
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileUpdate {
    pub id: String,
    pub status: FileStatus,
    pub progress: f32,
    pub processed_secs: Option<f64>,
    pub duration_secs: Option<f64>,
    pub new_segments: Option<Vec<EngineSegment>>,
    pub saved_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTranscript {
    pub id: String,
    pub name: String,
    pub source_path: String,
    pub segments: Vec<EngineSegment>,
    pub duration_secs: f64,
    pub saved_path: String,
}

#[derive(Default)]
pub struct QueueController {
    state: QueueState,
}

impl QueueController {
    pub fn start(&mut self, total: usize) -> Result<(), String> {
        if total == 0 {
            return Err("Нет файлов для расшифровки".into());
        }
        if matches!(
            self.state.status,
            QueueStatus::Running | QueueStatus::Paused
        ) {
            return Err("Очередь уже выполняется".into());
        }
        self.state = QueueState {
            status: QueueStatus::Running,
            overall_progress: 0.0,
            current_id: None,
            current_index: 0,
            total,
            completed: 0,
            failed: 0,
            message: None,
        };
        Ok(())
    }

    pub fn begin_file(&mut self, id: &str, index: usize) {
        self.state.current_id = Some(id.to_string());
        self.state.current_index = index;
        self.state.overall_progress = overall_progress(
            self.state.completed,
            self.state.failed,
            0.0,
            self.state.total,
        );
    }

    pub fn set_current_progress(&mut self, progress: f32) {
        self.state.overall_progress = overall_progress(
            self.state.completed,
            self.state.failed,
            progress,
            self.state.total,
        );
    }

    pub fn complete_file(&mut self) {
        self.state.completed += 1;
        self.advance();
    }

    pub fn fail_file(&mut self) {
        self.state.failed += 1;
        self.advance();
    }

    fn advance(&mut self) {
        self.state.current_id = None;
        self.state.current_index = self.state.completed + self.state.failed;
        self.state.overall_progress = overall_progress(
            self.state.completed,
            self.state.failed,
            0.0,
            self.state.total,
        );
        if self.state.completed + self.state.failed >= self.state.total {
            self.state.status = QueueStatus::Done;
            self.state.overall_progress = 100.0;
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        match (paused, self.state.status) {
            (true, QueueStatus::Running) => self.state.status = QueueStatus::Paused,
            (false, QueueStatus::Paused) => self.state.status = QueueStatus::Running,
            _ => {}
        }
    }

    pub fn set_message(&mut self, message: Option<String>) {
        self.state.message = message;
    }

    pub fn stop_with_error(&mut self, message: impl Into<String>) {
        self.state.status = QueueStatus::Error;
        self.state.message = Some(message.into());
        self.state.current_id = None;
    }

    pub fn state(&self) -> QueueState {
        self.state.clone()
    }
}

pub fn overall_progress(completed: usize, failed: usize, current: f32, total: usize) -> f32 {
    if total == 0 {
        return 0.0;
    }
    let processed = (completed + failed) as f32 + current.clamp(0.0, 100.0) / 100.0;
    ((processed / total as f32) * 100.0).clamp(0.0, 100.0)
}

pub struct QueueRuntime {
    controller: Mutex<QueueController>,
    pub pause: AtomicBool,
    pub running: AtomicBool,
    pub results: Mutex<HashMap<String, CompletedTranscript>>,
}

impl Default for QueueRuntime {
    fn default() -> Self {
        Self {
            controller: Mutex::new(QueueController::default()),
            pause: AtomicBool::new(false),
            running: AtomicBool::new(false),
            results: Mutex::new(HashMap::new()),
        }
    }
}

impl QueueRuntime {
    pub fn state(&self) -> QueueState {
        self.controller
            .lock()
            .map(|controller| controller.state())
            .unwrap_or_default()
    }

    pub fn start(&self, total: usize) -> Result<QueueState, String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("Очередь уже выполняется".into());
        }
        let mut controller = self.controller.lock().map_err(|_| "Очередь недоступна")?;
        if let Err(error) = controller.start(total) {
            self.running.store(false, Ordering::SeqCst);
            return Err(error);
        }
        self.pause.store(false, Ordering::SeqCst);
        Ok(controller.state())
    }

    pub fn set_paused(&self, paused: bool) -> Result<QueueState, String> {
        if !self.running.load(Ordering::SeqCst) {
            return Err("Очередь сейчас не выполняется".into());
        }
        self.pause.store(paused, Ordering::SeqCst);
        let mut controller = self.controller.lock().map_err(|_| "Очередь недоступна")?;
        controller.set_paused(paused);
        Ok(controller.state())
    }

    pub fn set_progress(&self, progress: f32) {
        if let Ok(mut controller) = self.controller.lock() {
            controller.set_current_progress(progress);
        }
    }

    pub fn set_message(&self, message: Option<String>) -> QueueState {
        let mut controller = self.controller.lock().expect("queue controller poisoned");
        controller.set_message(message);
        controller.state()
    }

    pub fn stop_with_error(&self, message: impl Into<String>) -> QueueState {
        self.running.store(false, Ordering::SeqCst);
        self.pause.store(false, Ordering::SeqCst);
        let mut controller = self.controller.lock().expect("queue controller poisoned");
        controller.stop_with_error(message);
        controller.state()
    }

    pub fn complete_file(&self) -> QueueState {
        let mut controller = self.controller.lock().expect("queue controller poisoned");
        controller.complete_file();
        controller.state()
    }

    pub fn fail_file(&self) -> QueueState {
        let mut controller = self.controller.lock().expect("queue controller poisoned");
        controller.fail_file();
        controller.state()
    }

    pub fn finish(&self) -> QueueState {
        self.running.store(false, Ordering::SeqCst);
        self.pause.store(false, Ordering::SeqCst);
        let controller = self.controller.lock().expect("queue controller poisoned");
        let mut state = controller.state();
        if state.completed + state.failed >= state.total && state.total > 0 {
            state.status = QueueStatus::Done;
            state.overall_progress = 100.0;
            state.current_id = None;
            state.current_index = state.total;
        }
        state
    }
}

fn emit_queue(app: &AppHandle, state: &QueueState) {
    let _ = app.emit("queue-state", state.clone());
}

fn emit_file(app: &AppHandle, update: &FileUpdate) {
    let _ = app.emit("file-update", update.clone());
}

fn wait_for_resume(
    runtime: &QueueRuntime,
    app: &AppHandle,
    file_id: &str,
    progress: f32,
    duration_secs: Option<f64>,
    processed_secs: Option<f64>,
) {
    if !runtime.pause.load(Ordering::SeqCst) {
        return;
    }
    emit_file(
        app,
        &FileUpdate {
            id: file_id.to_string(),
            status: FileStatus::Paused,
            progress,
            processed_secs,
            duration_secs,
            new_segments: None,
            saved_path: None,
            error: None,
        },
    );
    while runtime.pause.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(120));
    }
    emit_queue(app, &runtime.state());
    emit_file(
        app,
        &FileUpdate {
            id: file_id.to_string(),
            status: FileStatus::Processing,
            progress,
            processed_secs,
            duration_secs,
            new_segments: None,
            saved_path: None,
            error: None,
        },
    );
}

enum FileFailure {
    File(String),
    Fatal(String),
}

#[allow(clippy::too_many_arguments)]
pub fn run_batch(
    app: AppHandle,
    model: Arc<ModelManager>,
    audio: Arc<AudioEngine>,
    cache_dir: PathBuf,
    output_dir: PathBuf,
    jobs: Vec<FileJob>,
    runtime: Arc<QueueRuntime>,
) {
    emit_queue(
        &app,
        &runtime.set_message(Some("Подготовка аудиодвижка…".into())),
    );
    if let Err(error) = audio.ensure_ffmpeg() {
        let state = runtime.stop_with_error(format!("Не удалось подготовить FFmpeg: {error}"));
        emit_queue(&app, &state);
        return;
    }
    emit_queue(&app, &runtime.set_message(None));

    for (index, job) in jobs.iter().enumerate() {
        wait_for_resume(&runtime, &app, &job.id, 0.0, None, None);
        if let Ok(mut controller) = runtime.controller.lock() {
            controller.begin_file(&job.id, index);
        }
        emit_queue(&app, &runtime.state());
        emit_file(
            &app,
            &FileUpdate {
                id: job.id.clone(),
                status: FileStatus::Processing,
                progress: 0.0,
                processed_secs: None,
                duration_secs: None,
                new_segments: None,
                saved_path: None,
                error: None,
            },
        );

        match transcribe_file(&app, &model, &audio, &runtime, &cache_dir, &output_dir, job) {
            Ok(completed) => {
                if let Ok(mut results) = runtime.results.lock() {
                    results.insert(completed.id.clone(), completed.clone());
                }
                let state = runtime.complete_file();
                emit_file(
                    &app,
                    &FileUpdate {
                        id: completed.id,
                        status: FileStatus::Completed,
                        progress: 100.0,
                        processed_secs: Some(completed.duration_secs),
                        duration_secs: Some(completed.duration_secs),
                        new_segments: None,
                        saved_path: Some(completed.saved_path.clone()),
                        error: None,
                    },
                );
                emit_queue(&app, &state);
            }
            Err(FileFailure::File(error)) => {
                let state = runtime.fail_file();
                emit_file(
                    &app,
                    &FileUpdate {
                        id: job.id.clone(),
                        status: FileStatus::Error,
                        progress: 0.0,
                        processed_secs: None,
                        duration_secs: None,
                        new_segments: None,
                        saved_path: None,
                        error: Some(error),
                    },
                );
                emit_queue(&app, &state);
            }
            Err(FileFailure::Fatal(error)) => {
                runtime.fail_file();
                emit_file(
                    &app,
                    &FileUpdate {
                        id: job.id.clone(),
                        status: FileStatus::Error,
                        progress: 0.0,
                        processed_secs: None,
                        duration_secs: None,
                        new_segments: None,
                        saved_path: None,
                        error: Some(error.clone()),
                    },
                );
                let state = runtime.stop_with_error(error);
                emit_queue(&app, &state);
                return;
            }
        }
    }

    let state = runtime.finish();
    emit_queue(&app, &state);
}

fn transcribe_file(
    app: &AppHandle,
    model: &ModelManager,
    audio: &AudioEngine,
    runtime: &QueueRuntime,
    cache_dir: &Path,
    output_dir: &Path,
    job: &FileJob,
) -> Result<CompletedTranscript, FileFailure> {
    let temp = tempfile::Builder::new()
        .prefix("transaudio-")
        .tempdir_in(cache_dir)
        .map_err(|error| {
            FileFailure::File(format!("Не удалось создать временную папку: {error}"))
        })?;
    let wav_path = temp.path().join("audio.wav");
    audio
        .extract_wav(Path::new(&job.path), &wav_path)
        .map_err(FileFailure::File)?;
    let mut chunker = WavChunker::open(&wav_path).map_err(FileFailure::File)?;
    let total_samples = chunker.total_samples();
    if total_samples == 0 {
        return Err(FileFailure::File("В файле нет аудиодорожки".into()));
    }
    let duration_secs = chunker.duration_secs();
    runtime.set_progress(5.0);
    emit_queue(app, &runtime.state());
    emit_file(
        app,
        &FileUpdate {
            id: job.id.clone(),
            status: FileStatus::Processing,
            progress: 5.0,
            processed_secs: Some(0.0),
            duration_secs: Some(duration_secs),
            new_segments: None,
            saved_path: None,
            error: None,
        },
    );

    let mut all_segments = Vec::new();
    let mut last_progress = 5.0_f32;
    let mut last_processed = 0.0_f64;
    while let Some(chunk) = chunker.next_chunk().map_err(FileFailure::File)? {
        wait_for_resume(
            runtime,
            app,
            &job.id,
            last_progress,
            Some(duration_secs),
            Some(last_processed),
        );
        let result = model
            .transcribe(&chunk.samples)
            .map_err(|error| FileFailure::Fatal(error.to_string()))?;
        let offset = chunk.start_sample as f64 / SAMPLE_RATE as f64;
        let mut new_segments = Vec::new();
        for mut segment in result.segments {
            segment.start = (segment.start + offset).max(0.0);
            segment.end = (segment.end + offset).max(segment.start);
            if !segment.text.trim().is_empty() {
                new_segments.push(segment);
            }
        }
        if !result.text.trim().is_empty() && new_segments.is_empty() {
            new_segments.push(EngineSegment {
                start: offset,
                end: chunk.end_sample as f64 / SAMPLE_RATE as f64,
                text: result.text,
                speaker: None,
            });
        }
        all_segments.extend(new_segments.iter().cloned());
        let progress =
            (5.0 + 94.0 * (chunk.end_sample as f64 / total_samples as f64)).min(99.0) as f32;
        last_progress = progress;
        last_processed = chunk.end_sample as f64 / SAMPLE_RATE as f64;
        runtime.set_progress(progress);
        emit_queue(app, &runtime.state());
        emit_file(
            app,
            &FileUpdate {
                id: job.id.clone(),
                status: FileStatus::Processing,
                progress,
                processed_secs: Some(last_processed),
                duration_secs: Some(duration_secs),
                new_segments: Some(new_segments),
                saved_path: None,
                error: None,
            },
        );
    }

    if all_segments.is_empty() {
        return Err(FileFailure::File("Речь не распознана".into()));
    }

    let source_stem = Path::new(&job.name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(&job.name);
    let saved_path = export::write_result(
        output_dir,
        source_stem,
        ExportFormat::Md,
        &all_segments,
        duration_secs,
        &model.selected_name(),
    )
    .map_err(FileFailure::File)?
    .to_string_lossy()
    .to_string();

    Ok(CompletedTranscript {
        id: job.id.clone(),
        name: job.name.clone(),
        source_path: job.path.clone(),
        segments: all_segments,
        duration_secs,
        saved_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overall_progress_counts_finished_and_current_work() {
        assert_eq!(overall_progress(0, 0, 0.0, 4), 0.0);
        assert_eq!(overall_progress(1, 0, 0.0, 4), 25.0);
        assert_eq!(overall_progress(1, 0, 50.0, 4), 37.5);
        assert_eq!(overall_progress(1, 1, 0.0, 4), 50.0);
        assert_eq!(overall_progress(4, 0, 0.0, 4), 100.0);
    }

    #[test]
    fn controller_rejects_second_start_while_running() {
        let mut controller = QueueController::default();
        controller.start(2).unwrap();
        assert!(controller.start(2).is_err());
        assert_eq!(controller.state().status, QueueStatus::Running);
    }

    #[test]
    fn controller_tracks_file_completion_and_failure() {
        let mut controller = QueueController::default();
        controller.start(2).unwrap();
        controller.begin_file("a", 0);
        controller.set_current_progress(50.0);
        assert!((controller.state().overall_progress - 25.0).abs() < 0.1);
        controller.complete_file();
        controller.begin_file("b", 1);
        controller.fail_file();

        let state = controller.state();
        assert_eq!(state.completed, 1);
        assert_eq!(state.failed, 1);
        assert_eq!(state.current_index, 2);
        assert_eq!(state.status, QueueStatus::Done);
        assert_eq!(state.overall_progress, 100.0);
    }

    #[test]
    fn controller_pauses_and_resumes() {
        let mut controller = QueueController::default();
        controller.start(1).unwrap();
        controller.set_paused(true);
        assert_eq!(controller.state().status, QueueStatus::Paused);
        controller.set_paused(false);
        assert_eq!(controller.state().status, QueueStatus::Running);
    }
    #[test]
    fn controller_can_stop_with_a_visible_error() {
        let mut controller = QueueController::default();
        controller.start(2).unwrap();
        controller.begin_file("a", 0);
        controller.fail_file();
        controller.stop_with_error("FFmpeg недоступен");

        let state = controller.state();
        assert_eq!(state.status, QueueStatus::Error);
        assert_eq!(state.message.as_deref(), Some("FFmpeg недоступен"));
        assert!(
            controller.start(2).is_ok(),
            "a cleared queue can start again"
        );
    }
    #[test]
    fn starting_a_new_batch_keeps_finished_transcripts() {
        let runtime = QueueRuntime::default();
        runtime.results.lock().unwrap().insert(
            "done".into(),
            CompletedTranscript {
                id: "done".into(),
                name: "Готово.mp3".into(),
                source_path: "/tmp/Готово.mp3".into(),
                segments: Vec::new(),
                duration_secs: 10.0,
                saved_path: "/tmp/Готово.md".into(),
            },
        );

        runtime.start(1).unwrap();

        assert!(runtime.results.lock().unwrap().contains_key("done"));
        runtime.finish();
    }
}
