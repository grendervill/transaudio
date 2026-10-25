import type {
  Backend,
  Bootstrap,
  DiarizationStatus,
  ModelCard,
  DragDropEvent,
  FileUpdate,
  InspectedFile,
  ExportFormat,
  ExportResult,
  InspectFilesResult,
  ModelStatus,
  QueueState,
  Settings,
} from "../types";

const consentSettings: Settings = {
  output_dir: "/Users/test/Documents/Transaudio",
  model_consent: true,
  export_format: "md",
  model_id: "parakeet-tdt-0.6b-v3",
  canary_language: "ru",
  diarization: false,
  diarization_scenario: "interview",
  diarization_speakers: 0,
};

const readyModel: ModelStatus = {
  model_id: "parakeet-tdt-0.6b-v3",
  stage: "ready",
  progress: 100,
  downloaded_bytes: null,
  total_bytes: null,
  error: null,
};

const idleDiarization: DiarizationStatus = {
  stage: "not_downloaded",
  progress: 0,
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

export const sampleModels: ModelCard[] = [
  {
    id: "parakeet-tdt-0.6b-v3",
    name: "Parakeet TDT 0.6B v3",
    description: "Быстрая и точная.",
    family: "parakeet",
    languages: ["ru", "en", "de"],
    language_count: 25,
    quality_en: 4.83,
    quality_ru: 6.54,
    speed_x: 13.66,
    size_bytes: 739_508_576,
    license: "CC-BY-4.0",
    needs_language: false,
    recommended_for: [],
    badge: null,
    installed: true,
    selected: true,
  },
  {
    id: "gigaam-v3-e2e-rnnt",
    name: "GigaAM v3 E2E-RNNT",
    description: "Лучший выбор для русского.",
    family: "gigaam",
    languages: ["ru"],
    language_count: 1,
    quality_en: null,
    quality_ru: 5.35,
    speed_x: 7.88,
    size_bytes: 273_724_832,
    license: "MIT",
    needs_language: false,
    recommended_for: ["ru"],
    badge: null,
    installed: false,
    selected: false,
  },
];

export const defaultBootstrap: Bootstrap = {
  settings: consentSettings,
  model: readyModel,
  selected_model: "parakeet-tdt-0.6b-v3",
  models: sampleModels,
  diarization: idleDiarization,
  queue: idleQueue,
};

export function createFakeBackend(overrides: Partial<Backend> = {}): Backend {
  const backend: Backend = {
    bootstrap: async () => defaultBootstrap,
    inspectFiles: async (paths: string[]): Promise<InspectFilesResult> => ({
      files: paths.map<InspectedFile>((path) => ({
        id: `file-${path}`,
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
    listModels: async () => sampleModels,
    selectModel: async () => sampleModels,
    deleteModel: async () => sampleModels,
    setCanaryLanguage: async (language: string) => ({
      ...consentSettings,
      canary_language: language,
    }),
    setDiarization: async (
      enabled: boolean,
      _scenario: string,
      _speakers: number,
    ): Promise<DiarizationStatus> => ({
      stage: enabled ? "downloading" : "not_downloaded",
      progress: 0,
      error: null,
    }),
    setOutputDir: async (path: string) => ({
      ...consentSettings,
      output_dir: path,
    }),
    exportResults: async (ids: string[], _format: ExportFormat): Promise<ExportResult> => ({
      files_written: ids.map((id) => `/output/${id}.md`),
    }),
    openOutputDir: async () => {},
    pickFiles: async () => [],
    pickOutputDir: async () => null,
    onModelStatus: async (_callback: (status: ModelStatus) => void) => () => {},
    onFileUpdate: async (_callback: (update: FileUpdate) => void) => () => {},
    onQueueState: async (_callback: (state: QueueState) => void) => () => {},
    onDiarizationStatus: async (_callback: (status: DiarizationStatus) => void) => () => {},
    onDragDrop: async (_callback: (event: DragDropEvent) => void) => () => {},
    ...overrides,
  };
  return backend;
}
