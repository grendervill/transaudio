import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";

import type {
  Backend,
  Bootstrap,
  DiarizationStatus,
  DragDropEvent,
  ExportFormat,
  ExportResult,
  FileJob,
  FileUpdate,
  InspectFilesResult,
  ModelCard,
  ModelStatus,
  QueueState,
  Settings,
} from "../types";

const MEDIA_EXTENSIONS = [
  "mp3",
  "wav",
  "m4a",
  "flac",
  "aac",
  "ogg",
  "opus",
  "mp4",
  "mov",
  "mkv",
  "webm",
];

export function createTauriBackend(): Backend {
  return {
    bootstrap: () => invoke<Bootstrap>("bootstrap"),
    inspectFiles: (paths) =>
      invoke<InspectFilesResult>("inspect_files", { paths }),
    startTranscription: (files) =>
      invoke<QueueState>("start_transcription", { files }),
    setPaused: (paused) => invoke<QueueState>("set_paused", { paused }),
    grantConsent: (id, language) =>
      invoke<ModelStatus>("grant_model_consent", { id, language }),
    downloadModel: (id, language) =>
      invoke<ModelStatus>("download_model", { id, language }),
    listModels: () => invoke<ModelCard[]>("list_models"),
    selectModel: (id, language) =>
      invoke<ModelCard[]>("select_model", { id, language }),
    deleteModel: (id) => invoke<ModelCard[]>("delete_model", { id }),
    setCanaryLanguage: (language) =>
      invoke<Settings>("set_canary_language", { language }),
    setDiarization: (enabled, scenario, speakers) =>
      invoke<DiarizationStatus>("set_diarization", {
        enabled,
        scenario,
        speakers,
      }),
    setOutputDir: (path) => invoke<Settings>("set_output_dir", { path }),
    exportResults: (ids, format) =>
      invoke<ExportResult>("export_results", { ids, format }),
    openOutputDir: () => invoke<void>("open_output_dir"),
    async pickFiles() {
      const selected = await open({
        multiple: true,
        directory: false,
        filters: [{ name: "Аудио и видео", extensions: MEDIA_EXTENSIONS }],
      });
      if (!selected) return [];
      return Array.isArray(selected) ? selected : [selected];
    },
    async pickOutputDir() {
      const selected = await open({ directory: true, multiple: false });
      return typeof selected === "string" ? selected : null;
    },
    onModelStatus: (callback) =>
      listen<ModelStatus>("model-status", (event) => callback(event.payload)),
    onFileUpdate: (callback) =>
      listen<FileUpdate>("file-update", (event) => callback(event.payload)),
    onQueueState: (callback) =>
      listen<QueueState>("queue-state", (event) => callback(event.payload)),
    onDiarizationStatus: (callback) =>
      listen<DiarizationStatus>("diarization-status", (event) => callback(event.payload)),
    onDragDrop: async (callback) => {
      const unlisten = await getCurrentWebview().onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === "drop") {
          callback({ type: "drop", paths: payload.paths });
        } else {
          callback({ type: payload.type } as DragDropEvent);
        }
      });
      return unlisten;
    },
  };
}

export const tauriBackend = createTauriBackend();

export type { FileJob };
