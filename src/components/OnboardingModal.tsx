import { formatBytes } from "../lib/format";
import type { ModelStatus } from "../types";
import { Icon } from "./Icon";
import { Modal } from "./Modal";

interface OnboardingModalProps {
  open: boolean;
  model: ModelStatus;
  onAllow: () => void | Promise<void>;
  onRetry: () => void | Promise<void>;
  onDismiss: () => void;
}

function stageCopy(model: ModelStatus): string {
  switch (model.stage) {
    case "downloading":
      return "Загружаем модель. Это можно свернуть — загрузка продолжится.";
    case "loading":
      return "Запускаем локальный движок на этом компьютере.";
    case "error":
      return "Не удалось завершить настройку модели. Проверьте соединение и попробуйте ещё раз.";
    default:
      return "Для расшифровки нужна модель Parakeet V3. Скачаем её один раз, и приложение больше не будет спрашивать.";
  }
}

export function OnboardingModal({
  open,
  model,
  onAllow,
  onRetry,
  onDismiss,
}: OnboardingModalProps) {
  const busy = model.stage === "downloading" || model.stage === "loading";
  const progress = Math.round(model.progress);

  return (
    <Modal open={open} onClose={onDismiss} titleId="onboarding-title" className="welcome">
      <div className="welcome-graphic">
        <span className="model-badge mono">V3</span>
        <div className="signal" aria-hidden="true">
          {Array.from({ length: 44 }).map((_, index) => (
            <i key={index} style={{ height: `${28 + Math.abs(Math.sin(index * 0.63)) * 58}%` }} />
          ))}
        </div>
      </div>
      <p className="kicker">Первый запуск</p>
      <h2 id="onboarding-title">
        Одна загрузка.
        <br />
        Дальше — только ваш компьютер.
      </h2>
      <p className="modal-lead">{stageCopy(model)}</p>

      {model.stage === "consent_required" ? (
        <>
          <div className="permission-facts">
            <div>
              <span className="mono">≈ 640 МБ</span>
              <span>модель на диске</span>
            </div>
            <div>
              <span className="mono">1 раз</span>
              <span>разрешение</span>
            </div>
            <div>
              <span className="mono">0 КБ</span>
              <span>уходит в облако</span>
            </div>
          </div>
          <p className="small modal-note">
            Аудио, видео и текст остаются на этом компьютере. Для видео приложение один раз
            подготовит локальный FFmpeg.
          </p>
          <div className="modal-actions">
            <button className="button button-accent" type="button" onClick={() => void onAllow()}>
              <Icon name="download" />
              Разрешить загрузку
            </button>
            <button className="button button-quiet" type="button" onClick={onDismiss}>
              Не сейчас
            </button>
          </div>
        </>
      ) : null}

      {busy ? (
        <div className="model-progress">
          <div className="progress-copy">
            <span>{model.stage === "loading" ? "Запускаем Parakeet V3" : "Загружаем Parakeet V3"}</span>
            <span className="mono">
              {model.stage === "downloading"
                ? model.total_bytes
                  ? `${formatBytes(model.downloaded_bytes ?? 0)} / ${formatBytes(model.total_bytes)}`
                  : `${progress}%`
                : "подготовка"}
            </span>
          </div>
          <div
            className="progress-track"
            role="progressbar"
            aria-label="Загрузка модели"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={progress}
          >
            <i style={{ width: `${model.stage === "loading" ? 100 : progress}%` }} />
          </div>
          <div className="modal-actions">
            <button className="button button-quiet" type="button" onClick={onDismiss}>
              Свернуть
            </button>
          </div>
        </div>
      ) : null}

      {model.stage === "error" ? (
        <div className="model-error">
          <p>{model.error}</p>
          <div className="modal-actions">
            <button className="button button-accent" type="button" onClick={() => void onRetry()}>
              Повторить
            </button>
            <button className="button button-quiet" type="button" onClick={onDismiss}>
              Не сейчас
            </button>
          </div>
        </div>
      ) : null}
    </Modal>
  );
}
