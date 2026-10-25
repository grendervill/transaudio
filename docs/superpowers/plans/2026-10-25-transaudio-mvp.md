# Transaudio MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Собрать рабочее кроссплатформенное Tauri-приложение «Tape Room» для локальной пакетной расшифровки аудио и видео моделью Parakeet V3.

**Architecture:** React/TypeScript-интерфейс работает с Rust-ядром через Tauri commands и events. Rust-ядро хранит настройки, скачивает и загружает Parakeet ONNX, извлекает аудио через FFmpeg, нарезает его и последовательно расшифровывает очередь, сохраняя Markdown автоматически и поддерживая экспорт MD/SRT/TXT.

**Tech Stack:** Tauri 2.12, React 19, TypeScript, Vite, Rust, `transcribe-rs` 0.3.12, `ort` 2.0.0-rc.12, Parakeet TDT 0.6B v3 int8 ONNX, FFmpeg, Vitest + Testing Library.

**Spec:** `docs/superpowers/specs/2026-10-25-transaudio-design.md`

## Global Constraints

- Модель: Parakeet TDT 0.6B v3 int8, архив `https://blob.handy.computer/parakeet-v3-int8.tar.gz`, SHA-256 `43d37191602727524a7d8c6da0eef11c4ba24320f5b4730f1a2497befc2efa77`.
- `ort` зафиксирован на `=2.0.0-rc.12`; без этого `transcribe-rs 0.3.12` не собирается.
- Все аудио приводится к 16 кГц, mono, PCM s16le перед чтением модели.
- Один активный файл за раз; очередь строго последовательная.
- Первое разрешение на загрузку модели сохраняется в настройках; после него вопрос не повторяется.
- Модель запускается при старте приложения без ручного действия пользователя.
- В интерфейсе только русский язык; технические идентификаторы — английские.
- В расшифровке нет спикеров: модель их не предоставляет.
- Оригинальные медиафайлы не перемещаются и не изменяются.
- Выходные файлы пишутся в UTF-8; имена санитизируются для Windows/macOS/Linux.
- Поддерживаемые ОС: macOS, Windows, Linux. Пути только через `PathBuf`/Tauri path API.
- Визуальное направление: `mockups/01-tape-room.html`; локальные шрифты и Lucide-спрайт.

## Review Focus

- **Огромные файлы:** медиа на несколько часов не должно загружаться в память целиком до нарезки; проверьте, что приложение не падает по OOM и прогресс не замирает навсегда.
- **Пауза между окнами:** пауза не должна терять уже расшифрованные сегменты; проверьте, что после продолжения таймкоды не сдвигаются.
- **Сбой сети при загрузке модели:** не должно оставаться «полуустановленной» модели; проверьте, что приложение можно перезапустить и повторить загрузку.
- **Пути Windows:** пробелы, кириллица и `\` в путях не должны ломать FFmpeg и имена экспортируемых файлов.
- **Повторный запуск:** модель не скачивается снова при наличии готового каталога, даже если интернет отсутствует.

---

## Файловая структура

```text
package.json, vite.config.ts, tsconfig*.json, index.html
src/
  main.tsx, App.tsx, styles.css, types.ts
  lib/backend.ts, lib/format.ts, lib/format.test.ts
  components/Icon.tsx, Rail.tsx, Topbar.tsx, GlobalBar.tsx
  components/FilesScreen.tsx, TranscriptionScreen.tsx, ExportScreen.tsx
  components/OnboardingModal.tsx, SettingsModal.tsx, Toast.tsx
  test/setup.ts, test/test-utils.tsx
  components/*.test.tsx, App.test.tsx
public/fonts/, public/icons/lucide.svg, public/favicon.svg
src-tauri/
  Cargo.toml, build.rs, tauri.conf.json, capabilities/default.json
  src/main.rs, lib.rs, commands.rs, config.rs, model.rs, audio.rs, transcribe.rs, export.rs
tests/ — Rust unit tests внутри модулей + integration smoke
docs/ — спецификация, план, платформенная сборка
```

### Task 1: Каркас Tauri + React + тестовое окружение

**Files:**
- Create: `package.json`, `vite.config.ts`, `tsconfig.json`, `tsconfig.node.json`, `index.html`
- Create: `src/main.tsx`, `src/App.tsx`, `src/test/setup.ts`
- Create: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`
- Create: `src-tauri/icons/*` из `mockups/assets/favicon.svg`
- Test: `src/App.test.tsx`

**Interfaces:**
- Produces: запускаемое окно Tauri с React-корнем; скрипты `dev`, `build`, `tauri`, `test`.
- Consumes: ничего.

- [ ] **Step 1: Инициализировать npm-проект и зависимости**

Версии: React 19, Vite 8, TypeScript 7 или совместимый `~5.9`, Tauri CLI/API 2.12.x, Vitest, Testing Library. Добавить `uuid`, `@tauri-apps/plugin-dialog`.

- [ ] **Step 2: Написать падающий тест рендера оболочки**

```tsx
it('renders the app shell and three navigation items', () => {
  render(<App backend={fakeBackend} />);
  expect(screen.getByRole('navigation')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: /Файлы/ })).toBeInTheDocument();
});
```

- [ ] **Step 3: Запустить тест и убедиться, что он падает** — `npm test -- --run`.

- [ ] **Step 4: Создать минимальные `App`, `main.tsx`, Tauri-проект, конфиг окна 1440×900 / min 1024×720**

`src-tauri/Cargo.toml` содержит точные зависимости: `tauri = "2"`, `transcribe-rs = { version = "0.3.12", features = ["onnx"] }`, `ort = "=2.0.0-rc.12"`, `reqwest`, `sha2`, `flate2`, `tar`, `ffmpeg-sidecar`, `uuid`, `serde`, `serde_json`, `thiserror`.

- [ ] **Step 5: Запустить тест, `npm run build`, `cargo check`; исправить ошибки.**

- [ ] **Step 6: Коммит** — `git commit -m "chore: scaffold tauri react app"`.

### Task 2: Настройки, пути и базовые команды

**Files:**
- Create: `src-tauri/src/config.rs`, `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/config.rs` unit tests

**Interfaces:**
- Produces:
  - `config::Settings { output_dir: PathBuf, model_consent: bool, export_format: ExportFormat }`
  - `config::SettingsStore::load_or_default(app_data: &Path, documents: &Path) -> Result<SettingsStore>`
  - `SettingsStore::settings()`, `SettingsStore::save()`, `SettingsStore::set_output_dir(PathBuf)`
  - `bootstrap() -> BootstrapPayload`
  - `set_output_dir(path: String) -> Settings`
  - `open_output_dir()`

- [ ] **Step 1: Написать тесты для дефолтной папки, сохранения consent и нормализации output dir.**

- [ ] **Step 2: Тесты должны упасть** — `cargo test config`.

- [ ] **Step 3: Реализовать JSON-хранилище в `$APP_DATA/settings.json`, создание `Documents/Transaudio`, санитизацию пустого пути.**

- [ ] **Step 4: Зарегистрировать команды и bootstrap с актуальным статусом модели/очереди (пока заглушки типов).**

- [ ] **Step 5: `cargo test`, `npm run build`.**

- [ ] **Step 6: Коммит** — `git commit -m "feat: persist app settings"`.

### Task 3: Менеджер модели Parakeet V3

**Files:**
- Create: `src-tauri/src/model.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/src/commands.rs`
- Test: `src-tauri/src/model.rs`

**Interfaces:**
- Produces:
  - `ModelStatus { stage, progress, downloaded_bytes, total_bytes, error }`
  - `ModelManager::new(paths: ModelPaths, sink: StatusSink) -> Arc<ModelManager>`
  - `ModelManager::start_once()`, `status()`, `is_ready()`, `with_model(closure)`, `grant_and_download()`, `retry_download()`
  - `ModelStage::{ConsentRequired, Downloading, Loading, Ready, Error}`
  - Tauri events `model-status`; команды `grant_model_consent`, `download_model`.

- [ ] **Step 1: Тесты:** `is_installed` распознаёт только каталог со всеми четырьмя файлами; `status` начинается с `ConsentRequired`; при `model_consent=true` и отсутствующей модели переход в Downloading; при наличии — Loading/Ready (модель не загружать в юнит-тесте, использовать fake loader trait).
- [ ] **Step 2: Убедиться, что тесты падают.**
- [ ] **Step 3: Реализовать потоковую загрузку с `reqwest`, проверкой длины, SHA-256, безопасной распаковкой tar.gz во временный каталог, поиском папки модели и атомарным rename.**
- [ ] **Step 4: Реализовать загрузку `ParakeetModel` в отдельном потоке и `with_model`, эмитить статусы на каждом этапе.**
- [ ] **Step 5: `cargo test`, собрать с моделью и проверить загрузку на реальном архиве вручную.**
- [ ] **Step 6: Коммит** — `git commit -m "feat: download and load parakeet model"`.

### Task 4: Аудиодвижок, инспекция файлов и нарезка

**Files:**
- Create: `src-tauri/src/audio.rs`, `src-tauri/src/transcribe.rs` (только chunk splitter)
- Modify: `src-tauri/src/commands.rs`
- Test: unit tests в `audio.rs` и `transcribe.rs`

**Interfaces:**
- Produces:
  - `audio::SUPPORTED_EXTENSIONS`
  - `audio::inspect_file(path) -> Result<MediaFileInput, String>`
  - `AudioEngine::ensure_ffmpeg() -> Result<PathBuf, String>`
  - `audio::extract_wav(input, temp_dir) -> Result<PathBuf>`
  - `audio::read_samples(path) -> Result<Vec<f32>>`
  - `transcribe::split_chunks(samples: &[f32]) -> Vec<(usize, usize)>`
  - `inspect_files(paths: Vec<String>) -> InspectFilesResult`

- [ ] **Step 1: Тесты:** расширения принимаются регистронезависимо; не-файлы и неподдерживаемые расширения отклоняются; `split_chunks` режет аудио на отрезки 27–33 с, не теряет сэмплы и находит тихую точку.
- [ ] **Step 2: Убедиться, что тесты падают.**
- [ ] **Step 3: Реализовать поиск FFmpeg: `$APP_DATA/bin/ffmpeg`, затем PATH; при отсутствии — скачивание через `ffmpeg-sidecar` в `$APP_DATA/bin` и распаковка только `ffmpeg`.**
- [ ] **Step 4: Реализовать конвертацию в temp WAV, чтение hound в f32 16k mono, удаление temp после обработки.**
- [ ] **Step 5: Реализовать `inspect_files` с id, именем, расширением, размером и списком отклонённых.**
- [ ] **Step 6: `cargo test`; ручная конвертация MP3/MP4/MOV через тестовый файл.**
- [ ] **Step 7: Коммит** — `git commit -m "feat: audio extraction and chunking"`.

### Task 5: Форматирование и экспорт

**Files:**
- Create: `src-tauri/src/export.rs`
- Test: unit tests в `export.rs`

**Interfaces:**
- Produces:
  - `ExportFormat::{Md, Srt, Txt}` с serde `md|srt|txt`
  - `export::safe_stem(name: &str) -> String`
  - `export::srt_timestamp(secs: f64) -> String`
  - `export::render(format, name, segments, duration) -> String`
  - `write_result(output_dir, name, format, segments, duration) -> PathBuf`
- Consumes: `TranscriptSegment { start, end, text }`.

- [ ] **Step 1: Тесты:** SRT (`00:00:03,000 --> 00:00:07,500`), Markdown с заголовком и таймкодами, TXT без таймкодов; `safe_stem` заменяет `/\:*?"<>|`, обрезает длину, защищает Windows reserved names, кириллица сохраняется.
- [ ] **Step 2: Тесты падают.**
- [ ] **Step 3: Реализовать рендер и атомарную запись файлов.**
- [ ] **Step 4: `cargo test export`.**
- [ ] **Step 5: Коммит** — `git commit -m "feat: transcript export formats"`.

### Task 6: Очередь расшифровки, события и автосохранение

**Files:**
- Modify: `src-tauri/src/transcribe.rs`, `state.rs` (new), `commands.rs`, `lib.rs`
- Test: `src-tauri/src/transcribe.rs` unit tests + manual command test

**Interfaces:**
- Produces:
  - `start_transcription(files: Vec<FileJob>, output_dir: PathBuf) -> QueueState`
  - `set_paused(paused: bool) -> QueueState`
  - events `queue-state`, `file-update`
  - `QueueState { status, overall_progress, current_id, current_index, total, completed, failed }`
  - worker sequentially: temp audio → chunks → model → auto-save `.md`.

- [ ] **Step 1: Тесты:** формула общего прогресса учитывает завершённые, ошибочные и дробный прогресс текущего файла; второй `start_transcription` во время работы возвращает ошибку; пауза между файлами не запускает следующий файл.
- [ ] **Step 2: Тесты падают.**
- [ ] **Step 3: Реализовать состояние очереди, `AtomicBool` паузы, фоновой поток и `AppHandle::emit`.**
- [ ] **Step 4: Каждое окно: проверка паузы, inference, offset таймкодов, эмит частичных сегментов/прогресса.**
- [ ] **Step 5: По завершении файла — автосохранение Markdown, сохранение результата в map для экспорта, переход к следующему.**
- [ ] **Step 6: Ошибки файла эмитятся в `file-update`, очередь продолжается; ошибка модели/FFmpeg останавливает очередь с `status=done` и понятным сообщением.**
- [ ] **Step 7: `cargo test`; интеграционный прогон на синтезированном русском WAV (не коммитить модель/аудио).**
- [ ] **Step 8: Коммит** — `git commit -m "feat: sequential transcription queue"`.

### Task 7: Слой данных фронтенда и оболочка Tape Room

**Files:**
- Create: `src/types.ts`, `src/lib/backend.ts`, `src/lib/format.ts`, `src/styles.css`
- Modify: `src/App.tsx`, `src/main.tsx`
- Test: `src/lib/format.test.ts`, `src/App.test.tsx`

**Interfaces:**
- Produces:
  - `Backend` интерфейс с методами bootstrap/inspect/grant/start/pause/export/openFolder/setOutputDir и подписками на события.
  - `createTauriBackend(): Backend` и `createFakeBackend(overrides?): Backend` для тестов.
  - оболочка: rail, topbar, screens, globalbar, toast.
  - `formatDuration(secs)`, `formatBytes(n)`, `formatTimecode(secs)`.

- [ ] **Step 1: Тесты форматтеров и оболочки:** активный раздел, глобальный футер, названия кнопок; fake backend.
- [ ] **Step 2: Тесты падают.**
- [ ] **Step 3: Реализовать типы, backend-обёртку, mock для тестов и адаптивную оболочку из макета.**
- [ ] **Step 4: Подключить локальные шрифты, иконки, favicon; настроить CSS-переменные, размеры, reduced motion, focus-visible.**
- [ ] **Step 5: `npm test`, `npm run build`, визуальный скриншот в браузере/через Tauri.**
- [ ] **Step 6: Коммит** — `git commit -m "feat: tape room app shell"`.

### Task 8: Экран «Файлы»

**Files:**
- Create: `src/components/FilesScreen.tsx`
- Modify: `src/App.tsx`, `src/styles.css`
- Test: `src/components/FilesScreen.test.tsx`

**Interfaces:**
- Consumes: `MediaFile[]`, `ModelStatus`, callbacks `onAdd(paths)`, `onRemove(id)`, `onStart`, `onChooseFolder`, `onOpenSettings`.
- Produces: drag/drop через `getCurrentWebview().onDragDropEvent`, нативный multiple file picker, состояния disabled/ready/running.

- [ ] **Step 1: Тесты:** пустое состояние; кнопка «Начать» disabled без файлов/модели; добавление файлов рендерит строки; активная модель включает кнопку; running блокирует удаление.
- [ ] **Step 2: Тесты падают.**
- [ ] **Step 3: Реализовать экран и dropzone, диалог выбора файлов, отображение ошибок, папку, модель, старт.**
- [ ] **Step 4: `npm test`, `npm run build`; визуально проверить 1440×900 и 1024×768.**
- [ ] **Step 5: Коммит** — `git commit -m "feat: files screen with drag and drop"`.

### Task 9: Экран «Транскрипция» и живой прогресс

**Files:**
- Create: `src/components/TranscriptionScreen.tsx`, `src/components/GlobalBar.tsx`
- Modify: `src/App.tsx`, `src/styles.css`
- Test: `src/components/TranscriptionScreen.test.tsx`, `src/components/GlobalBar.test.tsx`

**Interfaces:**
- Consumes: queue/files, currentId, model status, `onPause(paused)`, `onCopy`, `onOpenFolder`.
- Produces: группировка файлов, активный прогресс под файлом, общий футер, сегменты с таймкодами, скрытие пустых состояний.

- [ ] **Step 1: Тесты:** активный файл показывает прогресс под собой; завершённый файл можно раскрыть; общий прогресс отображает 41% и ETA; pause переключает label.
- [ ] **Step 2: Тесты падают.**
- [ ] **Step 3: Реализовать экран и обновление по событиям бэкенда.**
- [ ] **Step 4: `npm test`, `npm run build`; визуальная проверка.**
- [ ] **Step 5: Коммит** — `git commit -m "feat: transcription screen with live progress"`.

### Task 10: Экран «Экспорт»

**Files:**
- Create: `src/components/ExportScreen.tsx`
- Modify: `src/App.tsx`, `src/lib/backend.ts`, `src/styles.css`
- Test: `src/components/ExportScreen.test.tsx`

**Interfaces:**
- Consumes: completed files, selected ids, format, outputDir, `onExport(ids, format)`.
- Produces: чекбоксы, выбор формата, вызов команды, уведомление о сохранении.

- [ ] **Step 1: Тесты:** пустое состояние; чекбоксы; disabled без выбора; корректный вызов экспорта; отображение итогового пути.
- [ ] **Step 2: Тесты падают.**
- [ ] **Step 3: Реализовать экран, состояния и обработку ошибок; добавить команду `export_results` в бэкенд.**
- [ ] **Step 4: `npm test`, `cargo test`; визуальная проверка.**
- [ ] **Step 5: Коммит** — `git commit -m "feat: export screen and writers"`.

### Task 11: Первый запуск, настройки и обработка ошибок

**Files:**
- Create: `src/components/OnboardingModal.tsx`, `src/components/SettingsModal.tsx`, `src/components/Modal.tsx`
- Modify: `src/App.tsx`, `src/lib/backend.ts`, `src/styles.css`
- Test: `src/components/OnboardingModal.test.tsx`, `src/components/SettingsModal.test.tsx`

**Interfaces:**
- Consumes: `ModelStatus`, `Settings`, consent callbacks.
- Produces: прогресс загрузки и загрузки модели, повтор при ошибке, нативная смена папки, открытие папки, блокировка фона.

- [ ] **Step 1: Тесты:** показывается без consent; после нажатия вызывает grant; прогресс; «Не сейчас» закрывает; при consent=false повторно не спрашивается после готовности; настройки показывают папку и статус.
- [ ] **Step 2: Тесты падают.**
- [ ] **Step 3: Реализовать модальные окна, focus trap, Escape, aria-modal.**
- [ ] **Step 4: `npm test`; ручная проверка первого запуска с удалённой моделью и с уже готовой.**
- [ ] **Step 5: Коммит** — `git commit -m "feat: onboarding and settings"`.

### Task 12: Сборка, кроссплатформенные инструкции и финальная проверка

**Files:**
- Modify: `README.md`, `docs/mockups.md` (уточнить, что макеты исторические)
- Create: `docs/building.md`, `.github/workflows/build.yml`
- Create: `src-tauri/icons/*` (полный набор)
- Test: полный прогон

**Interfaces:**
- Produces: инструкции для macOS/Windows/Linux, CI-матрица, готовый бинарник macOS.

- [ ] **Step 1: Добавить `docs/building.md` с зависимостями Tauri 2, FFmpeg, моделью и путями данных.**
- [ ] **Step 2: Добавить GitHub Actions: `macos-14`, `windows-latest`, `ubuntu-22.04`; установка Node/Rust/WebKitGTK, `npm ci`, `cargo test`, `npm run tauri build`.**
- [ ] **Step 3: Сгенерировать иконки приложения из `mockups/assets/favicon.svg`.**
- [ ] **Step 4: `npm ci && npm test && npm run build && cargo test && npm run tauri build`.**
- [ ] **Step 5: Запустить `.app` или dev-окно, проверить: первый запуск, загрузка модели, 2 файла, пауза, экспорт, перезапуск.**
- [ ] **Step 6: Визуальная проверка 1440×900 и 1024×768 через macOS screenshot.**
- [ ] **Step 7: Коммит** — `git commit -m "docs: cross-platform build and release setup"`.

## Self-review

- Все требования спецификации покрыты задачами 2–12.
- Нет задач, которые вводят функции вне scope: диаризация, редактирование текста и сохранение очереди явно исключены.
- Все межмодульные имена синхронизированы: `ModelStatus`, `QueueState`, `MediaFile`, `TranscriptSegment`, `ExportFormat`.
- Риски упаковки ONNX Runtime и FFmpeg вынесены в проверку Task 12 и Review Focus.
- План проверяет реальную модель на macOS, а не только компиляцию.
