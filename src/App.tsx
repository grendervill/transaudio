import { useCallback, useEffect, useState } from "react";

import { FilesScreen } from "./components/FilesScreen";
import { OnboardingModal } from "./components/OnboardingModal";
import { ExportScreen } from "./components/ExportScreen";
import { GlobalBar } from "./components/GlobalBar";
import { Rail } from "./components/Rail";
import { SettingsModal } from "./components/SettingsModal";
import { Toast } from "./components/Toast";
import { TranscriptionScreen } from "./components/TranscriptionScreen";
import { Topbar } from "./components/Topbar";
import type {
  Backend,
  Bootstrap,
  FileUpdate,
  ModelStatus,
  QueueFile,
  QueueState,
  Settings,
  ExportFormat,
  Screen,
} from "./types";

interface AppProps {
  backend: Backend;
}

function mergeFileUpdate(files: QueueFile[], update: FileUpdate): QueueFile[] {
  return files.map((file) => {
    if (file.id !== update.id) return file;
    return {
      ...file,
      status: update.status,
      progress: update.progress,
      duration_secs: update.duration_secs ?? file.duration_secs,
      processed_secs: update.processed_secs ?? file.processed_secs,
      segments: update.new_segments ? [...file.segments, ...update.new_segments] : file.segments,
      saved_path: update.saved_path ?? file.saved_path,
      error: update.error ?? file.error,
    };
  });
}

export default function App({ backend }: AppProps) {
  const [bootstrap, setBootstrap] = useState<Bootstrap | null>(null);
  const [screen, setScreen] = useState<Screen>("files");
  const [files, setFiles] = useState<QueueFile[]>([]);
  const [model, setModel] = useState<ModelStatus | null>(null);
  const [queue, setQueue] = useState<QueueState | null>(null);
  const [bootError, setBootError] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [onboardingOpen, setOnboardingOpen] = useState(false);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [toast, setToast] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    const cleanups: Array<() => void> = [];

    const subscribe = async () => {
      cleanups.push(
        await backend.onModelStatus((status) => {
          if (alive) setModel(status);
        }),
      );
      cleanups.push(
        await backend.onFileUpdate((update) => {
          if (alive) setFiles((current) => mergeFileUpdate(current, update));
        }),
      );
      cleanups.push(
        await backend.onQueueState((state) => {
          if (alive) setQueue(state);
        }),
      );
      const data = await backend.bootstrap();
      if (!alive) return;
      setBootstrap(data);
      setModel(data.model);
      setQueue(data.queue);
      setSettings(data.settings);
      setOnboardingOpen(data.model.stage === "consent_required");
    };

    subscribe().catch((error: unknown) => {
      if (alive) setBootError(error instanceof Error ? error.message : String(error));
    });

    return () => {
      alive = false;
      cleanups.forEach((cleanup) => cleanup());
    };
  }, [backend]);

  useEffect(() => {
    if (model?.stage === "ready") setOnboardingOpen(false);
  }, [model?.stage]);

  const handlePause = useCallback(
    async (paused: boolean) => {
      try {
        const state = await backend.setPaused(paused);
        setQueue(state);
      } catch (error) {
        console.error(error);
      }
    },
    [backend],
  );

  if (bootError) {
    return (
      <div className="boot-screen">
        <div className="boot-mark is-error">!</div>
        <h1>Не удалось запустить приложение</h1>
        <p>{bootError}</p>
      </div>
    );
  }

  if (!bootstrap || !model || !queue || !settings) {
    return (
      <div className="boot-screen">
        <div className="boot-mark">
          <svg className="icon" aria-hidden="true">
            <use href="/icons/lucide.svg#audio-waveform" />
          </svg>
        </div>
        <p className="kicker">Transaudio</p>
        <h1>Запускаем локальный движок</h1>
        <p>Parakeet V3 остаётся на этом компьютере.</p>
      </div>
    );
  }

  const notify = (message: string) => setToast(message);

  const addPaths = async (paths: string[]) => {
    if (queue.status === "running" || queue.status === "paused") {
      notify("Дождитесь завершения текущей очереди.");
      return;
    }
    try {
      const result = await backend.inspectFiles(paths);
      const accepted = result.files.filter(
        (candidate) => !files.some((file) => file.path === candidate.path),
      );
      if (accepted.length) {
        setFiles((current) => [
          ...current,
          ...accepted.map<QueueFile>((file) => ({
            ...file,
            status: "queued",
            progress: 0,
            processed_secs: null,
            duration_secs: null,
            segments: [],
            saved_path: null,
            error: null,
          })),
        ]);
      }
      if (result.rejected.length) {
        const first = result.rejected[0];
        notify(
          result.rejected.length === 1
            ? `${first.path}: ${first.reason}`
            : `${result.rejected.length} файла не добавлены. ${first.reason}`,
        );
      } else if (accepted.length === 0 && result.files.length > 0) {
        notify("Эти файлы уже есть в очереди.");
      }
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    }
  };

  const chooseFolder = async () => {
    try {
      const selected = await backend.pickOutputDir();
      if (!selected) return;
      const updated = await backend.setOutputDir(selected);
      setSettings(updated);
      notify(`Папка результатов: ${updated.output_dir}`);
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    }
  };

  const allowModel = async () => {
    try {
      const status = await backend.grantConsent();
      setModel(status);
      setSettings((current) =>
        current ? { ...current, model_consent: true } : current,
      );
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    }
  };

  const retryModel = async () => {
    try {
      const status = await backend.downloadModel();
      setModel(status);
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    }
  };

  const exportResults = async (ids: string[], format: ExportFormat) => {
    try {
      const result = await backend.exportResults(ids, format);
      notify(`Сохранено файлов: ${result.files_written.length}.`);
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
      throw error;
    }
  };

  const openOutputDir = async () => {
    try {
      await backend.openOutputDir();
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    }
  };

  const start = async () => {
    if (!files.length) return;
    try {
      setFiles((current) =>
        current.map((file) => ({
          ...file,
          status: "queued",
          progress: 0,
          processed_secs: null,
          segments: [],
          saved_path: null,
          error: null,
        })),
      );
      const state = await backend.startTranscription(
        files.map((file) => ({ id: file.id, path: file.path, name: file.name })),
      );
      setQueue(state);
      setScreen("transcribe");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div className="app">
      <Rail
        screen={screen}
        model={model}
        onNavigate={setScreen}
        onOpenSettings={() => setSettingsOpen(true)}
      />
      <div className="shell">
        <Topbar
          fileCount={files.length}
          model={model}
          onOpenSettings={() => setSettingsOpen(true)}
        />
        <main className="content">
          {screen === "files" && (
            <FilesScreen
              backend={backend}
              files={files}
              model={model}
              outputDir={settings.output_dir}
              queueStatus={queue.status}
              onAddPaths={addPaths}
              onRemoveFile={(id) =>
                setFiles((current) => current.filter((file) => file.id !== id))
              }
              onStart={start}
              onChooseFolder={chooseFolder}
              onOpenSettings={() => setSettingsOpen(true)}
              onNotify={notify}
            />
          )}
          {screen === "transcribe" && (
            <TranscriptionScreen
              files={files}
              queue={queue}
              model={model}
              onPause={handlePause}
              onOpenSettings={() => setSettingsOpen(true)}
              onNotify={notify}
            />
          )}
          {screen === "export" && (
            <ExportScreen
              completed={files.filter((file) => file.status === "completed")}
              outputDir={settings.output_dir}
              initialFormat={settings.export_format}
              onChooseFolder={chooseFolder}
              onOpenFolder={openOutputDir}
              onExport={exportResults}
            />
          )}
        </main>
        <GlobalBar queue={queue} files={files} model={model} onPause={handlePause} />
      </div>
      <OnboardingModal
        open={onboardingOpen && model.stage !== "ready"}
        model={model}
        onAllow={allowModel}
        onRetry={retryModel}
        onDismiss={() => {
          setOnboardingOpen(false);
          if (model.stage === "consent_required") {
            notify("Модель не загружена. Разрешение можно дать в настройках.");
          }
        }}
      />
      <SettingsModal
        open={settingsOpen}
        settings={settings}
        model={model}
        onClose={() => setSettingsOpen(false)}
        onChooseFolder={chooseFolder}
        onOpenFolder={openOutputDir}
        onAllowModel={allowModel}
        onRetryModel={retryModel}
      />
      <Toast message={toast} onDone={() => setToast(null)} />
    </div>
  );
}
