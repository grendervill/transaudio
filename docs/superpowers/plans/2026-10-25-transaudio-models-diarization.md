# Transaudio 0.2 Implementation Plan

> **For the implementer:** execute tasks in order. Each task starts with a
> failing test, then implementation, then a green suite and a commit.

**Spec:** `docs/superpowers/specs/2026-10-25-transaudio-models-diarization.md`
**Base:** `main` at 0.1.1 (`abb7bc4`)

## Global constraints

- ASR engine: `transcribe-cpp 0.4.1`, GGUF Q8_0 models from `handy-computer`.
- Diarization: `sherpa-onnx 2.0.0`, pyannote segmentation + CAM++.
- No ONNX Runtime / transcribe-rs after the migration.
- Model metadata (repo, revision, file, size, SHA-256, WER, xRT, languages)
  comes from Handy `catalog.json`; the values are in the spec table.
- Diarization default: off; scenario default `interview`.
- Ready files are never re-sent; results are keyed by file id and preserved.
- UI language is Russian.

## Files

```
src-tauri/src/catalog.rs        model catalog + download URLs
src-tauri/src/model.rs          rewritten: selected model, GGUF download, engine
src-tauri/src/diarize.rs        download + diarize + turn post-processing
src-tauri/src/transcribe.rs     queue, turn/chunk transcription, progress
src-tauri/src/export.rs         speaker-aware MD/SRT/TXT
src-tauri/src/config.rs         new settings with serde defaults
src-tauri/src/commands.rs       model/diarization commands
src-tauri/src/lib.rs            wiring
src/components/ModelPicker.tsx  cards, bars, badges, filters
src/components/OnboardingModal.tsx
src/components/SettingsModal.tsx
src/components/FilesScreen.tsx
src/components/TranscriptionScreen.tsx
src/lib/modelStatus.ts          revision reconciliation (keep)
src/types.ts, src/lib/backend.ts, src/App.tsx, src/styles.css
```

## Task 1: queue, preserved results, click-to-view

**Tests first**
- `FilesScreen.test.tsx`: adding a queued file while queue status is `done`
  enables «Начать»; only pending files are passed to `onStart`; completed rows
  call `onSelectFile`.
- `App.test.tsx`: start sends only files whose status is not `completed`;
  clicking a completed file switches to the transcript screen with that file.
- Rust `transcribe.rs`: `QueueRuntime::start` keeps previously completed results.

**Implementation**
- Remove `results.clear()` from `QueueRuntime::start`.
- `FilesScreen`: `pending = files.filter(status !== "completed")`; start enabled
  if `pending.length > 0 && model ready && !busy`; start handler receives
  `pending`. Button label «Все файлы расшифрованы» when no pending.
- Row click selects the file (`onSelectFile(id)`), remove button stops
  propagation.
- `App`: `selectedId` state; `start` maps only pending; after start select the
  first pending; `onSelectFile` sets selection + screen `transcribe`.
- `TranscriptionScreen`: reader shows the selected file (active, completed,
  queued, error); left rows are buttons; keep live progress under the active
  row.

**Done:** `npm test`, `cargo test`; commit `feat: keep finished transcripts and open them on click`.

## Task 2: transcribe-cpp engine, catalog, downloads

**Cargo**
- Remove `transcribe-rs`, `ort`; add `transcribe-cpp = "0.4.1"`.
- Keep `hound`, `reqwest`, `sha2`, `flate2`, `tar`, `tempfile`, `time`.

**Catalog tests**
- 12 ids, unique; every URL contains repository/revision/filename; every
  `size_bytes`/`sha256` non-empty; `recommended_for` only `en`/`ru`.

**Catalog module**
- `ModelSpec { id, name, description, family, languages, quality_en, quality_ru,
  speed_x, size_bytes, filename, repository, revision, sha256, license,
  needs_language, recommended_for }`
- `pub static MODELS`, `find(id)`, `download_url(spec)`.
- IDs and numbers exactly as in the spec table.

**Model manager tests**
- `is_installed` requires the GGUF file with the exact size.
- `begin_download` publishes `Downloading` immediately and bumps revision.
- `install_file` rejects a wrong SHA and leaves no model dir; accepts the right
  SHA and renames the file into `models/<id>`.
- selecting a non-installed model sets `NotInstalled` without loading.
- selecting an installed model loads through a fake loader and reaches `Ready`.
- language hint is passed to the loader.

**Model manager**
- `ModelStatus { model_id, stage, progress, downloaded_bytes, total_bytes,
  error, revision }`; stages `ConsentRequired | NotInstalled | Downloading |
  Loading | Ready | Error`.
- `ModelPaths { models_dir, cache_dir, diarization_dir }`.
- `selected_id: Mutex<String>`, `language: Mutex<Option<String>>`.
- Download: temp file in cache, stream with 120 ms progress, verify SHA-256,
  `create_dir_all(models/<id>)`, rename, remove old ONNX dir when `id ==
  parakeet-tdt-0.6b-v3`.
- `ModelEngine` = transcribe-cpp `Model` + `Session` + language; `transcribe`
  maps `Transcript.segments` (ms → s) into `EngineSegment`; empty segments are
  allowed.
- Panic guard and revision logic are kept from 0.1.1.

**Commands**
- `list_models`, `select_model(id)`, `download_model(id)`, `delete_model(id)`,
  `set_canary_language(lang)`.
- `bootstrap` includes `models`, `selected_model`, `diarization`.
- `grant_model_consent` saves consent and starts the selected model download.

**Done:** `cargo test`, `npm test` (unchanged UI still compiles); commit
`feat: switch to transcribe-cpp and model catalog`.

## Task 3: model picker UI

**Tests**
- `ModelPicker.test.tsx`: 12 cards; RU/EN badges on the right cards; speed and
  quality bars reflect the numbers (e.g. Parakeet 110M speed 5, Whisper large
  speed 1); filter «Русский» hides EN-only models; clicking «Скачать» calls
  `onDownload(id)`; selected card shows «Выбрана»; installed non-selected shows
  «Удалить».
- `OnboardingModal.test.tsx`: cards visible, Parakeet selected by default,
  button shows the selected size, consent triggers selection + download.
- `App.test.tsx`: selecting a model updates the sidebar and persists via backend.

**Implementation**
- `ModelPicker`: grid 2 columns, filters, card layout, 5-segment bars, badges
  top-right, languages line («Русский, английский и ещё 23»), WER line, size,
  action button. Canary language select inside the card.
- `OnboardingModal`: uses `ModelPicker`, consent + selected model download,
  progress.
- `SettingsModal`: model section opens the picker; diarization settings here
  too.
- `FilesScreen`: model summary + «Сменить».
- API calls and optimistic selected state.

**Done:** tests green; visual check of the picker at 1440×900 and 1024×768;
commit `feat: model catalog UI`.

## Task 4: diarization backend

**Tests**
- `scenario_threshold`: interview 0.45, meeting 0.35, many 0.30, auto 0.40.
- `merge_turns`: gap ≤0.4 s merged; turns <0.5 s absorbed; 0.15 s padding;
  max 30 s; speaker ids preserved.
- `assign_speaker(turn, segment)` / turn-to-segment mapping: segment fully in a
  turn gets it; overlapping gets the max-overlap speaker.
- install of the segmentation tar.bz2 and embedding onnx with SHA-256.

**Implementation**
- `diarize.rs`: `DiarizationManager` (status/revision/sink), `ensure_ready`
  (download with progress), `Diarizer::new`, `diarize(samples, scenario)`.
- Segmentation archive extracted with `bzip2` + `tar`; only `model.onnx` kept.
- Install to `models/diarization/{segmentation.onnx,embedding.onnx}`.
- `set_diarization(enabled, scenario)` persists and starts the model download
  when enabled; `diarization-status` events.
- `bootstrap` includes diarization status.

**Done:** `cargo test`; commit `feat: local speaker diarization`.

## Task 5: turn-based transcription + speakers in UI/export

**Tests**
- `transcribe.rs`: when diarization is on, each turn is transcribed and the
  resulting segments carry the turn speaker and offset; timestamp-less engine
  output falls back to one segment per turn; progress is monotonic.
- `export.rs`: MD/SRT/TXT include speaker labels; files without speakers are
  unchanged.
- `TranscriptionScreen.test.tsx`: speaker chips rendered; selected completed
  file shows speakers.
- `start_transcription` rejects when diarization is enabled but models are not
  ready; `ensure` runs before the queue.

**Implementation**
- `transcribe_file`: if diarization on → `read_wav_samples` (cap 2 h), diarize,
  merge/pad, transcribe slices ≤30 s, offset + speaker; else streaming
  `WavChunker` as today.
- Progress based on processed seconds; emit partial segments.
- `EngineSegment.speaker: Option<u32>`; UI chips/colors; grouping.
- Export labels.

**Done:** tests green; commit `feat: speaker labels in transcript and export`.

## Task 6: E2E, CI, release 0.2.0

- Bump `package.json`, `Cargo.toml`, `tauri.conf.json` to `0.2.0`.
- `npm test`, `cargo test`, clippy, `npm run tauri build`.
- Manual macOS E2E:
  - first run with a fresh profile: choose GigaAM, download, Russian file;
  - Parakeet v3 after migration (GGUF), add file B after A finished, only B runs;
  - click A shows its transcript;
  - diarization on a two-voice dialogue: 2 speakers, labels in UI and SRT;
  - Whisper turbo on Russian with a language hint.
- Visual QA at 1440×900 and 1024×768 (picker, files, transcription).
- Push, wait for five-target CI, download artifacts, publish v0.2.0 with
  SHA256SUMS and migration notes.
