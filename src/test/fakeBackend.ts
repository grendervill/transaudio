import type {
  Backend,
  Bootstrap,
  DragDropEvent,
  FileUpdate,
  InspectedFile,
  InspectFilesResult,
  ModelStatus,
  QueueState,
  Settings,
} from "../types";

const consentSettings: Settings = {
  output_dir: "/Users/test/Documents/Transaudio",
  model_consent: true,
  export_format: "md",
};

const readyModel: ModelStatus = {
  stage: "ready",
  progress: 100,
  downloaded_bytes: null,
  total_bytes: null,
  error: null,
};

const idleQueue: QueueState = {
  status: "idle",
  overall_progress: 0,
  current_id: null,
  current_index: 0,
  total: 0,
  completed: 0,
  failed: 0,
};

export const defaultBootstrap: Bootstrap = {
  settings: consentSettings,
  model: readyModel,
  queue: idleQueue,
};

export function createFakeBackend(overrides: Partial<Backend> = {}): Backend {
  const backend: Backend = {
    bootstrap: async () => defaultBootstrap,
    inspectFiles: async (paths: string[]): Promise<InspectFilesResult> => ({
      files: paths.map<InspectedFile>((path, index) => ({
        id: `file-${index}`,
        path,
        name: path.split(/[\\/]/).pop() ?? path,
        extension: "MP3",
        size_bytes: 1024,
      })),
      rejected: [],
    }),
    startTranscription: async () => idleQueue,
    setPaused: async () => idleQueue,
    grantConsent: async () => readyModel,
    downloadModel: async () => readyModel,
    setOutputDir: async (path: string) => ({
      ...consentSettings,
      output_dir: path,
    }),
    pickFiles: async () => [],
    pickOutputDir: async () => null,
    onModelStatus: async (_callback: (status: ModelStatus) => void) => () => {},
    onFileUpdate: async (_callback: (update: FileUpdate) => void) => () => {},
    onQueueState: async (_callback: (state: QueueState) => void) => () => {},
    onDragDrop: async (_callback: (event: DragDropEvent) => void) => () => {},
    ...overrides,
  };
  return backend;
}
