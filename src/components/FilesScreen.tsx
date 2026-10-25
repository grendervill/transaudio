import { useEffect, useRef, useState } from "react";

import { formatBytes, formatDuration } from "../lib/format";
import type { Backend, ModelStatus, QueueFile, QueueStatus } from "../types";
import { Icon } from "./Icon";

interface FilesScreenProps {
  backend: Backend;
  files: QueueFile[];
  model: ModelStatus;
  outputDir: string;
  queueStatus: QueueStatus;
  onAddPaths: (paths: string[]) => void;
  onRemoveFile: (id: string) => void;
  onStart: () => void;
  onSelectFile: (id: string) => void;
  onClear: () => void;
  onChooseFolder: () => void;
  onOpenSettings: () => void;
  onNotify?: (message: string) => void;
}

function stateLabel(file: QueueFile): string {
  switch (file.status) {
    case "completed":
      return "Готово";
    case "processing":
      return "Расшифровка";
    case "paused":
      return "Пауза";
    case "error":
      return "Ошибка";
    default:
      return "В очереди";
  }
}

function StateDot({ status }: { status: QueueFile["status"] }) {
  return <span className={`state-dot state-${status}`} aria-hidden="true" />;
}

function FileGlyph({ extension }: { extension: string }) {
  const video = ["MP4", "MOV", "MKV", "WEBM"].includes(extension);
  return <Icon name={video ? "file-play" : "file-audio"} />;
}

export function FilesScreen({
  backend,
  files,
  model,
  outputDir,
  queueStatus,
  onAddPaths,
  onRemoveFile,
  onStart,
  onSelectFile,
  onClear,
  onChooseFolder,
  onOpenSettings,
  onNotify,
}: FilesScreenProps) {
  const [dragging, setDragging] = useState(false);
  const busy = queueStatus === "running" || queueStatus === "paused";
  const pending = files.filter(
    (file) => file.status === "queued" || file.status === "error",
  );
  const completedCount = files.filter((file) => file.status === "completed").length;
  const nothingPending = files.length > 0 && pending.length === 0;
  const canStart = pending.length > 0 && model.stage === "ready" && !busy;
  const totalBytes = files.reduce((sum, file) => sum + file.size_bytes, 0);
  const knownDurations = files.reduce((sum, file) => sum + (file.duration_secs ?? 0), 0);
  const latest = useRef({ busy, onAddPaths, onNotify });
  latest.current = { busy, onAddPaths, onNotify };

  useEffect(() => {
    let alive = true;
    let cleanup: (() => void) | undefined;
    backend
      .onDragDrop((event) => {
        if (!alive) return;
        if (event.type === "enter" || event.type === "over") {
          setDragging(true);
        } else {
          setDragging(false);
        }
        if (event.type === "drop") {
          const callbacks = latest.current;
          if (callbacks.busy) {
            callbacks.onNotify?.("Добавить файлы можно после завершения текущей очереди.");
            return;
          }
          if (event.paths.length) callbacks.onAddPaths(event.paths);
        }
      })
      .then((unlisten) => {
        cleanup = unlisten;
      })
      .catch((error) => {
        console.error(error);
      });
    return () => {
      alive = false;
      cleanup?.();
    };
  }, [backend]);

  async function pickFiles() {
    try {
      const paths = await backend.pickFiles();
      if (paths.length) onAddPaths(paths);
    } catch (error) {
      onNotify?.(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <section className="screen files-screen" aria-label="Файлы">
      <div className="files-head">
        <div>
          <p className="kicker">Вход в проект</p>
          <h1>Добавьте записи</h1>
          <p className="lead">
            Аудио и видео обрабатываются по очереди. Всё остаётся на компьютере.
          </p>
        </div>
        <button
          className="button button-accent"
          type="button"
          onClick={pickFiles}
          disabled={busy}
        >
          <Icon name="plus" />
          Добавить файлы
        </button>
      </div>

      <div className="files-grid">
        <div className="files-main">
          <div
            className={`dropzone${dragging ? " is-dragging" : ""}${busy ? " is-busy" : ""}`}
            data-dropzone
            onDragOver={(event) => event.preventDefault()}
          >
            <span className="drop-mark">
              <Icon name="upload" />
            </span>
            <div className="drop-copy">
              <strong>Перетащите аудио или видео</strong>
              <span>MP3, WAV, M4A, FLAC, MP4, MOV, MKV — до 8 ГБ на файл</span>
            </div>
            <button
              className="button button-quiet"
              type="button"
              onClick={pickFiles}
              disabled={busy}
            >
              Выбрать файлы
            </button>
          </div>

          <section className="queue-card" aria-labelledby="queue-title">
            <div className="section-head">
              <h2 id="queue-title">Очередь</h2>
              <span className="mono muted">
                {files.length === 0
                  ? "пусто"
                  : `${files.length} ${files.length === 1 ? "файл" : "файла"}`}
              </span>
            </div>
            {files.length === 0 ? (
              <div className="queue-empty">
                <Icon name="audio-lines" />
                <strong>Пока ни одной записи</strong>
                <span>Добавьте аудио или видео — они появятся здесь.</span>
              </div>
            ) : (
              <ol className="file-list">
                {files.map((file, index) => (
                  <li
                    key={file.id}
                    className={`file-row file-${file.status}`}
                  >
                    <button
                      className="file-select"
                      type="button"
                      onClick={() => onSelectFile(file.id)}
                      aria-label={`Открыть расшифровку ${file.name}`}
                    >
                      <span className="file-index mono">
                        {String(index + 1).padStart(2, "0")}
                      </span>
                      <span className="file-kind">
                        <FileGlyph extension={file.extension} />
                      </span>
                      <span className="file-copy">
                        <strong>{file.name}</strong>
                        <span>
                          {file.extension} ·{" "}
                          {file.duration_secs
                            ? `${formatDuration(file.duration_secs)} · `
                            : ""}
                          {formatBytes(file.size_bytes)}
                        </span>
                        {file.error ? (
                          <span className="file-error-message">{file.error}</span>
                        ) : null}
                      </span>
                      <span className={`file-status status-${file.status}`}>
                        <StateDot status={file.status} />
                        {stateLabel(file)}
                      </span>
                    </button>
                    <button
                      className="row-action"
                      type="button"
                      aria-label={`Убрать ${file.name}`}
                      disabled={busy}
                      onClick={() => onRemoveFile(file.id)}
                    >
                      <Icon name="x" />
                    </button>
                  </li>
                ))}
              </ol>
            )}
            <div className="start-row">
              <span className="start-hint">
                {busy
                  ? "Очередь уже выполняется"
                  : files.length === 0
                    ? "Добавьте файлы, чтобы начать"
                    : pending.length === 0
                      ? `${completedCount} готово · новых файлов нет`
                      : `${pending.length} осталось · ${completedCount} готово · ${formatBytes(totalBytes)}${knownDurations ? ` · ${formatDuration(knownDurations)}` : ""}`}
              </span>
              <div className="start-actions">
                {files.length > 0 && !busy ? (
                  <button className="text-button clear-button" type="button" onClick={onClear}>
                    Очистить
                  </button>
                ) : null}
                <button
                  className="button button-accent button-start"
                  type="button"
                  onClick={onStart}
                  disabled={!canStart}
                >
                  <Icon name={nothingPending ? "check" : "play"} />
                  {nothingPending ? "Все файлы расшифрованы" : "Начать"}
                </button>
              </div>
            </div>
          </section>
        </div>

        <aside className="files-aside">
          <section className="aside-block">
            <p className="aside-label">Папка результатов</p>
            <div className="folder-line">
              <Icon name="folder" />
              <span className="mono path">{outputDir}</span>
            </div>
            <button className="text-button" type="button" onClick={onChooseFolder}>
              Изменить папку <Icon name="arrow-up-right" />
            </button>
          </section>
          <section className="aside-block model-block">
            <div className="model-badge mono">V3</div>
            <div>
              <p className="aside-label">Локальная модель</p>
              <strong>Parakeet</strong>
              <p className="small muted">
                {model.stage === "ready"
                  ? "Загружена · работает офлайн"
                  : model.stage === "error"
                    ? "Ошибка загрузки"
                    : model.stage === "consent_required"
                      ? "Модель не загружена"
                      : `Подготовка · ${Math.round(model.progress)}%`}
              </p>
            </div>
            <span
              className={`ready-dot ${model.stage === "ready" ? "is-ready" : ""}`}
              aria-hidden="true"
            />
          </section>
          <section className="aside-block">
            <p className="aside-label">Пока идёт работа</p>
            <p className="small">
              Очередь обрабатывает записи одна за другой. Если поставить паузу,
              текущий фрагмент закончится и место в очереди сохранится.
            </p>
            <button className="text-button" type="button" onClick={onOpenSettings}>
              Параметры модели <Icon name="settings" />
            </button>
          </section>
          <div className="side-note mono">TA // DESKTOP<br />LOCAL FIRST</div>
        </aside>
      </div>
    </section>
  );
}
