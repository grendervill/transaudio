import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";

import type {
  Backend,
  Bootstrap,
  DragDropEvent,
  FileJob,
  FileUpdate,
  InspectFilesResult,
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
    grantConsent: () => invoke<ModelStatus>("grant_model_consent"),
    downloadModel: () => invoke<ModelStatus>("download_model"),
    setOutputDir: (path) => invoke<Settings>("set_output_dir", { path }),
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
