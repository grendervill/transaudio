import type { ModelStatus, QueueFile, QueueState } from "../types";
import { formatDuration } from "../lib/format";
import { Icon } from "./Icon";

interface GlobalBarProps {
  queue: QueueState;
  files: QueueFile[];
  model: ModelStatus;
  onPause: (paused: boolean) => void;
}

function titleFor(queue: QueueState, files: QueueFile[]): string {
  if (queue.status === "idle") return "Очередь готова";
  if (queue.status === "done") {
    return queue.failed > 0 ? `Готово с ошибками · ${queue.failed}` : "Все записи готовы";
  }
  const current = files.find((file) => file.id === queue.current_id);
  return current?.name ?? "Подготовка очереди";
}

export function GlobalBar({ queue, files, model, onPause }: GlobalBarProps) {
  const isRunning = queue.status === "running";
  const isPaused = queue.status === "paused";
  const canPause = isRunning || isPaused;
  const current = files.find((file) => file.id === queue.current_id);
  const done = queue.completed + queue.failed;
  const overall = Math.round(queue.overall_progress);

  return (
    <footer className="globalbar" aria-label="Общий прогресс очереди">
      <div className="global-file">
        <span className={`live-dot ${queue.status}`} />
        <div className="global-titles">
          <span className="global-kicker mono">
            {queue.status === "idle"
              ? "Сейчас"
              : queue.status === "done"
                ? "Итог"
                : queue.current_id
                  ? `Сейчас · файл ${Math.min(queue.current_index + 1, queue.total)} из ${queue.total}`
                  : "Сейчас"}
          </span>
          <strong>{titleFor(queue, files)}</strong>
        </div>
      </div>
      <div className="global-progress">
        <div className="global-progress-copy">
          <span>Все файлы</span>
          <span className="mono">
            {overall}%{queue.total > 0 ? ` · ${done} из ${queue.total} готово` : ""}
            {current?.duration_secs ? ` · ${formatDuration(current.duration_secs)}` : ""}
          </span>
        </div>
        <div
          className="global-track"
          role="progressbar"
          aria-label="Общий прогресс очереди"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={overall}
        >
          <i style={{ width: `${overall}%` }} />
        </div>
      </div>
      <div className="global-actions">
        <span className={`model-foot ${model.stage}`}>
          <i />
          {model.stage === "ready" ? "модель локально" : "модель не готова"}
        </span>
        <button
          className="transport"
          type="button"
          data-pause
          disabled={!canPause}
          aria-pressed={isPaused}
          onClick={() => onPause(!isPaused)}
        >
          <Icon name={isPaused ? "play" : "pause"} />
          <span>{isPaused ? "Продолжить" : "Пауза"}</span>
        </button>
      </div>
    </footer>
  );
}
