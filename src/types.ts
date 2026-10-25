export type Screen = "files" | "transcribe" | "export";
export type ExportFormat = "md" | "srt" | "txt";

export type ModelStage =
  | "consent_required"
  | "not_installed"
  | "downloading"
  | "loading"
  | "ready"
  | "error";

export interface ModelStatus {
  model_id: string;
  stage: ModelStage;
  progress: number;
  downloaded_bytes: number | null;
  total_bytes: number | null;
  error: string | null;
  revision?: number;
}

export type ModelFamily =
  | "parakeet"
  | "canary"
  | "gigaam"
  | "whisper"
  | "sensevoice"
  | "qwen";

export interface ModelCard {
  id: string;
  name: string;
  description: string;
  family: ModelFamily;
  languages: string[];
  language_count: number;
  quality_en: number | null;
  quality_ru: number | null;
  speed_x: number;
  size_bytes: number;
  license: string;
  needs_language: boolean;
  recommended_for: string[];
  badge: string | null;
  installed: boolean;
  selected: boolean;
}

export interface Settings {
  output_dir: string;
  model_consent: boolean;
  export_format: ExportFormat;
  model_id: string;
  canary_language: string;
  diarization: boolean;
  diarization_scenario: string;
}

export type QueueStatus = "idle" | "running" | "paused" | "done" | "error";

export interface QueueState {
  status: QueueStatus;
  overall_progress: number;
  current_id: string | null;
  current_index: number;
  total: number;
  completed: number;
  failed: number;
  message?: string | null;
}

export interface Bootstrap {
  settings: Settings;
  model: ModelStatus;
  selected_model: string;
  models: ModelCard[];
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
  speaker?: number | null;
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

export interface ExportResult {
  files_written: string[];
}

export interface Backend {
  bootstrap(): Promise<Bootstrap>;
  inspectFiles(paths: string[]): Promise<InspectFilesResult>;
  startTranscription(files: FileJob[]): Promise<QueueState>;
  setPaused(paused: boolean): Promise<QueueState>;
  grantConsent(id?: string, language?: string): Promise<ModelStatus>;
  downloadModel(id: string, language?: string): Promise<ModelStatus>;
  listModels(): Promise<ModelCard[]>;
  selectModel(id: string, language?: string): Promise<ModelCard[]>;
  deleteModel(id: string): Promise<ModelCard[]>;
  setCanaryLanguage(language: string): Promise<Settings>;
  setOutputDir(path: string): Promise<Settings>;
  exportResults(ids: string[], format: ExportFormat): Promise<ExportResult>;
  openOutputDir(): Promise<void>;
  pickFiles(): Promise<string[]>;
  pickOutputDir(): Promise<string | null>;
  onModelStatus(callback: (status: ModelStatus) => void): Promise<() => void>;
  onFileUpdate(callback: (update: FileUpdate) => void): Promise<() => void>;
  onQueueState(callback: (state: QueueState) => void): Promise<() => void>;
  onDragDrop(callback: (event: DragDropEvent) => void): Promise<() => void>;
}
