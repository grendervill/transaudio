import { useEffect, useMemo } from "react";

import { formatDuration, formatTimecode } from "../lib/format";
import type { EngineSegment, ModelStatus, QueueFile, QueueState } from "../types";
import { Icon } from "./Icon";

interface TranscriptionScreenProps {
  files: QueueFile[];
  queue: QueueState;
  model: ModelStatus;
  modelName: string;
  selectedId: string | null;
  onSelectFile: (id: string) => void;
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
      return "готово";
    case "processing":
      return "текущий файл";
    case "paused":
      return "пауза";
    case "error":
      return "ошибка";
    default:
      return "в очереди";
  }
}

function SegmentList({ segments, live }: { segments: EngineSegment[]; live: boolean }) {
  if (segments.length === 0) return null;
  return (
    <div className="transcript">
      {segments.map((segment, index) => (
        <div
          key={`${segment.start}-${index}`}
          className={`segment${live && index === segments.length - 1 ? " is-current" : ""}`}
        >
          <time className="mono">{formatTimecode(segment.start)}</time>
          <div>
            {segment.speaker ? (
              <span
                className={`speaker-chip speaker-${((segment.speaker - 1) % 6) + 1}`}
              >
                Спикер {segment.speaker}
              </span>
            ) : null}
            <p>{segment.text}</p>
            {live && index === segments.length - 1 ? (
              <span className="cursor" aria-hidden="true" />
            ) : null}
          </div>
        </div>
      ))}
    </div>
  );
}

function Reader({
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
  const active = isActive(file);
  const paused = file.status === "paused" || queue.status === "paused";

  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      onNotify?.("Текст расшифровки скопирован.");
    } catch {
      onNotify?.("Не удалось скопировать текст.");
    }
  }

  function emptyState() {
    if (file.status === "error") {
      return (
        <div className="file-error-block" role="alert">
          <Icon name="triangle-alert" />
          <span>{file.error ?? "Не удалось расшифровать файл."}</span>
        </div>
      );
    }
    if (file.status === "processing") {
      return (
        <div className="transcript-empty">
          <span className="live-dot running" />
          <span>Слушаю запись…</span>
        </div>
      );
    }
    if (file.status === "paused") {
      return (
        <div className="transcript-empty">
          <span className="live-dot paused" />
          <span>Расшифровка на паузе.</span>
        </div>
      );
    }
    if (file.status === "completed") {
      return (
        <div className="transcript-empty">
          <span className="live-dot" />
          <span>Речь не распознана.</span>
        </div>
      );
    }
    return (
      <div className="transcript-empty">
        <span className="live-dot" />
        <span>
          Файл ещё не расшифрован. Нажмите «Начать» на экране файлов.
        </span>
      </div>
    );
  }

  return (
    <article className="file-block is-live" id={`transcript-${file.id}`}>
      <div className="reading-meta">
        <span className="mono">
          Файл {index + 1} / {files.length}
        </span>
        <span
          className={`live-label${active ? "" : " is-idle"}${paused && active ? " is-paused" : ""}`}
        >
          {active ? <i /> : null}
          {file.status === "processing" && !paused
            ? "идёт расшифровка"
            : file.status === "paused" || (active && paused)
              ? "пауза"
              : statusLabel(file)}
        </span>
      </div>
      <div className="title-row">
        <h1>{file.name.replace(/\.[^.]+$/, "")}</h1>
        <div className="head-actions">
          <button
            className="icon-button"
            type="button"
            onClick={copy}
            aria-label="Скопировать текст"
            disabled={file.segments.length === 0}
          >
            <Icon name="copy" />
          </button>
          <button
            className="icon-button"
            type="button"
            onClick={onOpenSettings}
            aria-label="Настройки"
          >
            <Icon name="settings" />
          </button>
        </div>
      </div>
      <p className="file-meta mono">
        {file.extension}
        {file.duration_secs ? ` · ${formatDuration(file.duration_secs)}` : ""}
        {file.segments.length ? ` · ${file.segments.length} фрагм.` : ""}
        {file.saved_path ? " · сохранено" : ""}
      </p>
      <div className="reading-rule" />
      {file.segments.length > 0 ? (
        <SegmentList segments={file.segments} live={active && !paused} />
      ) : (
        emptyState()
      )}
    </article>
  );
}

export function TranscriptionScreen({
  files,
  queue,
  model: _model,
  modelName,
  selectedId,
  onSelectFile,
  onOpenSettings,
  onNotify,
}: TranscriptionScreenProps) {
  const activeFile = files.find((file) => isActive(file));
  const selected = useMemo(
    () =>
      files.find((file) => file.id === selectedId) ??
      activeFile ??
      files.at(-1) ??
      null,
    [files, selectedId, activeFile],
  );
  const overall = Math.round(queue.overall_progress);

  useEffect(() => {
    if (!selected) return;
    const element = document.getElementById(`transcript-${selected.id}`);
    if (element && typeof element.scrollIntoView === "function") {
      element.scrollIntoView({ block: "start", behavior: "smooth" });
    }
  }, [selected?.id]);

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
              <h2>
                {files.length} {files.length === 1 ? "запись" : "записи"}
              </h2>
              <span className="mono">{overall}%</span>
            </div>
          </div>
          <ol className="reel-list">
            {files.map((file, index) => {
              const active = isActive(file);
              const isSelected = selected?.id === file.id;
              return (
                <li
                  key={file.id}
                  className={`reel reel-${file.status}${active ? " is-current" : ""}${
                    isSelected ? " is-selected" : ""
                  }`}
                >
                  <button
                    className="reel-button"
                    type="button"
                    aria-current={isSelected ? "true" : undefined}
                    onClick={() => onSelectFile(file.id)}
                  >
                    <span className="reel-number mono">
                      {String(index + 1).padStart(2, "0")}
                    </span>
                    <span className="reel-copy">
                      <strong>{file.name}</strong>
                      <span>{statusLabel(file)}</span>
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
                  </button>
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
            <span className="mono muted">{modelName} · локально</span>
            <span className="mono muted">{files.length ? `${overall}%` : "—"}</span>
          </div>
        </aside>

        <div className="reader">
          <div className="reader-inner">
            {selected ? (
              <Reader
                file={selected}
                files={files}
                queue={queue}
                onOpenSettings={onOpenSettings}
                onNotify={onNotify}
              />
            ) : (
              <div className="empty-reader">
                <Icon name="captions" />
                <h1>Здесь появится расшифровка.</h1>
                <p>Добавьте записи и нажмите «Начать».</p>
              </div>
            )}
            {selected && selected.status === "completed" && selected.segments.length > 0 ? (
              <div className="reader-foot-actions">
                <button
                  className="text-button"
                  type="button"
                  onClick={() => copyText(selected.segments.map((s) => s.text).join("\n\n"))}
                >
                  <Icon name="copy" /> Скопировать всё
                </button>
              </div>
            ) : null}
          </div>
        </div>
      </div>
    </section>
  );
}
