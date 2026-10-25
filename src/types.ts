export type Screen = "files" | "transcribe" | "export";
export type ExportFormat = "md" | "srt" | "txt";

export type ModelStage =
  | "consent_required"
  | "downloading"
  | "loading"
  | "ready"
  | "error";

export interface ModelStatus {
  stage: ModelStage;
  progress: number;
  downloaded_bytes: number | null;
  total_bytes: number | null;
  error: string | null;
}

export interface Settings {
  output_dir: string;
  model_consent: boolean;
  export_format: ExportFormat;
}

export type QueueStatus = "idle" | "running" | "paused" | "done";

export interface QueueState {
  status: QueueStatus;
  overall_progress: number;
  current_id: string | null;
  current_index: number;
  total: number;
  completed: number;
  failed: number;
}

export interface Bootstrap {
  settings: Settings;
  model: ModelStatus;
  queue: QueueState;
}

export interface InspectedFile {
  id: string;
  path: string;
  name: string;
  extension: string;
  size_bytes: number;
}

export interface RejectedFile {
  path: string;
  reason: string;
}

export interface InspectFilesResult {
  files: InspectedFile[];
  rejected: RejectedFile[];
}

export type FileStatus = "queued" | "processing" | "paused" | "completed" | "error";

export interface EngineSegment {
  start: number;
  end: number;
  text: string;
}

export interface FileUpdate {
  id: string;
  status: FileStatus;
  progress: number;
  processed_secs: number | null;
  duration_secs: number | null;
  new_segments: EngineSegment[] | null;
  saved_path: string | null;
  error: string | null;
}

export interface FileJob {
  id: string;
  path: string;
  name: string;
}

export interface QueueFile extends InspectedFile {
  status: FileStatus;
  progress: number;
  processed_secs: number | null;
  duration_secs: number | null;
  segments: EngineSegment[];
  saved_path: string | null;
  error: string | null;
}

export type DragDropEvent =
  | { type: "enter"; paths?: undefined }
  | { type: "over"; paths?: undefined }
  | { type: "drop"; paths: string[] }
  | { type: "leave"; paths?: undefined };

export interface Backend {
  bootstrap(): Promise<Bootstrap>;
  inspectFiles(paths: string[]): Promise<InspectFilesResult>;
  startTranscription(files: FileJob[]): Promise<QueueState>;
  setPaused(paused: boolean): Promise<QueueState>;
  grantConsent(): Promise<ModelStatus>;
  downloadModel(): Promise<ModelStatus>;
  setOutputDir(path: string): Promise<Settings>;
  pickFiles(): Promise<string[]>;
  pickOutputDir(): Promise<string | null>;
  onModelStatus(callback: (status: ModelStatus) => void): Promise<() => void>;
  onFileUpdate(callback: (update: FileUpdate) => void): Promise<() => void>;
  onQueueState(callback: (state: QueueState) => void): Promise<() => void>;
  onDragDrop(callback: (event: DragDropEvent) => void): Promise<() => void>;
}
