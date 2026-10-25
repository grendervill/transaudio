import { useEffect, useMemo, useState } from "react";

import { formatDuration, formatTimecode } from "../lib/format";
import type { ModelStatus, QueueFile, QueueState, EngineSegment } from "../types";
import { Icon } from "./Icon";

interface TranscriptionScreenProps {
  files: QueueFile[];
  queue: QueueState;
  model: ModelStatus;
  onPause: (paused: boolean) => void;
  onOpenSettings: () => void;
  onNotify?: (message: string) => void;
}

function isActive(file: QueueFile): boolean {
  return file.status === "processing" || file.status === "paused";
}

function statusLabel(file: QueueFile): string {
  switch (file.status) {
    case "completed":
      return "Готово";
    case "processing":
      return "Идёт расшифровка";
    case "paused":
      return "Пауза";
    case "error":
      return "Ошибка";
    default:
      return "В очереди";
  }
}

function SegmentList({
  segments,
  live,
  emptyLabel,
}: {
  segments: EngineSegment[];
  live: boolean;
  emptyLabel?: string;
}) {
  if (segments.length === 0) {
    return (
      <div className="transcript-empty">
        <span className={`live-dot ${live ? "running" : "paused"}`} />
        <span>{emptyLabel ?? (live ? "Слушаю запись…" : "Текст не распознан.")}</span>
      </div>
    );
  }
  return (
    <div className="transcript">
      {segments.map((segment, index) => (
        <div
          key={`${segment.start}-${index}`}
          className={`segment${live && index === segments.length - 1 ? " is-current" : ""}`}
        >
          <time className="mono">{formatTimecode(segment.start)}</time>
          <div>
            <p>{segment.text}</p>
            {live && index === segments.length - 1 ? <span className="cursor" aria-hidden="true" /> : null}
          </div>
        </div>
      ))}
    </div>
  );
}

function ActiveFile({
  file,
  files,
  queue,
  onOpenSettings,
  onNotify,
}: {
  file: QueueFile;
  files: QueueFile[];
  queue: QueueState;
  onOpenSettings: () => void;
  onNotify?: (message: string) => void;
}) {
  const index = files.findIndex((candidate) => candidate.id === file.id);
  const text = file.segments.map((segment) => segment.text).join("\n\n");
  const paused = file.status === "paused" || queue.status === "paused";

  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      onNotify?.("Текст расшифровки скопирован.");
    } catch {
      onNotify?.("Не удалось скопировать текст.");
    }
  }

  return (
    <article className="file-block is-live" id={`transcript-${file.id}`}>
      <div className="reading-meta">
        <span className="mono">
          Файл {index + 1} / {files.length}
        </span>
        <span className={`live-label${paused ? " is-paused" : ""}`}>
          <i />
          {paused ? "Пауза" : "идёт расшифровка"}
        </span>
      </div>
      <div className="title-row">
        <h1>{file.name.replace(/\.[^.]+$/, "")}</h1>
        <div className="head-actions">
          <button className="icon-button" type="button" onClick={copy} aria-label="Скопировать текст">
            <Icon name="copy" />
          </button>
          <button className="icon-button" type="button" onClick={onOpenSettings} aria-label="Настройки">
            <Icon name="settings" />
          </button>
        </div>
      </div>
      <p className="file-meta mono">
        {file.extension} · {file.duration_secs ? formatDuration(file.duration_secs) : "длительность уточняется"} ·{" "}
        {Math.round(file.progress)}%
      </p>
      <div className="reading-rule" />
      <SegmentList
        segments={file.segments}
        live={!paused}
        emptyLabel={paused ? "Расшифровка на паузе." : undefined}
      />
    </article>
  );
}

function CompletedFile({
  file,
  files,
  onCopy,
  defaultOpen = false,
}: {
  file: QueueFile;
  files: QueueFile[];
  onCopy: (text: string) => void;
  defaultOpen?: boolean;
}) {
  const [open, setOpen] = useState(defaultOpen);
  useEffect(() => {
    if (defaultOpen) setOpen(true);
  }, [defaultOpen]);
  const index = files.findIndex((candidate) => candidate.id === file.id);
  const text = file.segments.map((segment) => segment.text).join("\n\n");

  return (
    <article className={`file-block is-complete${open ? " is-open" : ""}`} id={`transcript-${file.id}`}>
      <button
        className="file-summary"
        type="button"
        aria-expanded={open}
        aria-label={`${file.name}, ${statusLabel(file)}`}
        onClick={() => setOpen((value) => !value)}
      >
        <span className="summary-index mono">{String(index + 1).padStart(2, "0")}</span>
        <span className="summary-copy">
          <strong>{file.name.replace(/\.[^.]+$/, "")}</strong>
          <span className="summary-meta mono">
            {file.duration_secs ? formatDuration(file.duration_secs) : "готово"} ·{" "}
            {file.segments.length} фрагм.
            {file.saved_path ? " · сохранено" : ""}
          </span>
        </span>
        <span className={`status-tag${file.status === "error" ? " is-error" : " is-done"}`}>
          {statusLabel(file)}
        </span>
        <Icon name={open ? "chevron-down" : "chevron-right"} className="summary-chevron" />
      </button>
      {open ? (
        <div className="completed-body">
          {file.status === "error" && file.error ? (
            <div className="file-error-block" role="alert">
              <Icon name="triangle-alert" />
              <span>{file.error}</span>
            </div>
          ) : (
            <>
              <div className="completed-tools">
                <button className="text-button" type="button" onClick={() => onCopy(text)}>
                  <Icon name="copy" /> Скопировать
                </button>
              </div>
              <SegmentList segments={file.segments} live={false} />
            </>
          )}
        </div>
      ) : null}
    </article>
  );
}

export function TranscriptionScreen({
  files,
  queue,
  model: _model,
  onOpenSettings,
  onNotify,
}: TranscriptionScreenProps) {
  const activeFile = files.find((file) => isActive(file));
  const completedFiles = useMemo(
    () => files.filter((file) => file.status === "completed" || file.status === "error"),
    [files],
  );
  const visible = [...completedFiles, ...(activeFile ? [activeFile] : [])];
  const overall = Math.round(queue.overall_progress);
  const lastCompletedId = completedFiles.at(-1)?.id;

  useEffect(() => {
    if (!queue.current_id) return;
    const element = document.getElementById(`transcript-${queue.current_id}`);
    if (element && typeof element.scrollIntoView === "function") {
      element.scrollIntoView({ block: "start", behavior: "smooth" });
    }
  }, [queue.current_id]);

  useEffect(() => {
    if (queue.status !== "done" || !lastCompletedId) return;
    const element = document.getElementById(`transcript-${lastCompletedId}`);
    if (element && typeof element.scrollIntoView === "function") {
      element.scrollIntoView({ block: "start", behavior: "smooth" });
    }
  }, [queue.status, lastCompletedId]);

  async function copyText(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      onNotify?.("Текст расшифровки скопирован.");
    } catch {
      onNotify?.("Не удалось скопировать текст.");
    }
  }

  return (
    <section className="screen transcribe-screen" aria-label="Транскрипция">
      <div className="transcribe-grid">
        <aside className="queue-pane">
          <div className="pane-head">
            <p className="kicker">Очередь</p>
            <div className="pane-title-row">
              <h2>{files.length} {files.length === 1 ? "запись" : "записи"}</h2>
              <span className="mono">{overall}%</span>
            </div>
          </div>
          <ol className="reel-list">
            {files.map((file, index) => {
              const active = isActive(file);
              const effectiveStatus =
                active && queue.status === "paused" ? "paused" : file.status;
              return (
                <li
                  key={file.id}
                  className={`reel reel-${file.status}${active ? " is-current" : ""}`}
                >
                  <div className="reel-button">
                    <span className="reel-number mono">
                      {String(index + 1).padStart(2, "0")}
                    </span>
                    <span className="reel-copy">
                      <strong>{file.name}</strong>
                      <span>
                        {effectiveStatus === "completed"
                          ? "готово"
                          : effectiveStatus === "processing"
                            ? "текущий файл"
                            : effectiveStatus === "paused"
                              ? "пауза"
                              : effectiveStatus === "error"
                                ? "ошибка"
                                : "в очереди"}
                      </span>
                    </span>
                    {active ? (
                      <span className="mini-percent mono">{Math.round(file.progress)}%</span>
                    ) : file.status === "completed" ? (
                      <Icon name="check" />
                    ) : file.status === "error" ? (
                      <Icon name="triangle-alert" />
                    ) : (
                      <Icon name="clock" />
                    )}
                  </div>
                  {active ? (
                    <div className="file-progress">
                      <div className="file-progress-copy">
                        <span>Прогресс файла</span>
                        <span className="mono">
                          {file.duration_secs
                            ? `${formatDuration(file.processed_secs ?? 0)} / ${formatDuration(file.duration_secs)}`
                            : "подготовка"}
                        </span>
                      </div>
                      <div
                        className="mini-track"
                        role="progressbar"
                        aria-label="Прогресс текущего файла"
                        aria-valuemin={0}
                        aria-valuemax={100}
                        aria-valuenow={Math.round(file.progress)}
                      >
                        <i style={{ width: `${file.progress}%` }} />
                      </div>
                      <div className="file-progress-foot">
                        <span>
                          {queue.status === "paused" ? "пауза" : "осталось примерно"}
                          {file.duration_secs
                            ? ` ${formatDuration(Math.max(0, file.duration_secs - (file.processed_secs ?? 0)))}`
                            : ""}
                        </span>
                        <span className="mono">16 kHz</span>
                      </div>
                    </div>
                  ) : null}
                </li>
              );
            })}
          </ol>
          <div className="queue-foot">
            <span className="mono muted">Parakeet V3 · локально</span>
            <span className="mono muted">{files.length ? `${overall}%` : "—"}</span>
          </div>
        </aside>

        <div className="reader">
          <div className="reader-inner">
            {visible.length === 0 ? (
              <div className="empty-reader">
                <Icon name="captions" />
                <h1>Здесь появится расшифровка.</h1>
                <p>Добавьте записи и нажмите «Начать».</p>
              </div>
            ) : (
              visible.map((file) =>
                isActive(file) ? (
                  <ActiveFile
                    key={file.id}
                    file={file}
                    files={files}
                    queue={queue}
                    onOpenSettings={onOpenSettings}
                    onNotify={onNotify}
                  />
                ) : (
                  <CompletedFile
                    key={file.id}
                    file={file}
                    files={files}
                    onCopy={copyText}
                    defaultOpen={queue.status === "done" && file.id === lastCompletedId}
                  />
                ),
              )
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
