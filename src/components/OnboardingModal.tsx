import type { ModelCard, ModelStatus } from "../types";
import { Icon } from "./Icon";
import { Modal } from "./Modal";
import { ModelPicker } from "./ModelPicker";

interface OnboardingModalProps {
  open: boolean;
  models: ModelCard[];
  status: ModelStatus;
  canaryLanguage: string;
  onSelect: (id: string) => void;
  onDownload: (id: string) => void;
  onLanguage: (language: string) => void;
  onRetry: () => void | Promise<void>;
  onDismiss: () => void;
}

export function OnboardingModal({
  open,
  models,
  status,
  canaryLanguage,
  onSelect,
  onDownload,
  onLanguage,
  onRetry,
  onDismiss,
}: OnboardingModalProps) {
  const busy = status.stage === "downloading" || status.stage === "loading";
  const selected = models.find((card) => card.selected);

  return (
    <Modal open={open} onClose={onDismiss} titleId="onboarding-title" className="welcome is-wide">
      <div className="welcome-graphic">
        <span className="model-badge mono">TA</span>
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
      <p className="modal-lead">
        Выберите модель под свои записи. Скачается только она; остальные можно
        добавить позже в настройках. Аудио и текст остаются на этом компьютере.
      </p>

      <ModelPicker
        models={models}
        status={status}
        canaryLanguage={canaryLanguage}
        busy={busy}
        onSelect={onSelect}
        onDownload={onDownload}
        onDelete={() => {}}
        onLanguage={onLanguage}
      />

      <div className="modal-actions welcome-actions">
        <span className="small muted">
          {selected
            ? `Выбрано: ${selected.name}. Разрешение на загрузку сохранится.`
            : "Выберите модель, чтобы начать."}
        </span>
        <button className="button button-quiet" type="button" onClick={onDismiss}>
          Не сейчас
        </button>
      </div>

      {status.stage === "error" ? (
        <div className="model-error">
          <p>{status.error}</p>
          <div className="modal-actions">
            <button className="button button-accent" type="button" onClick={() => void onRetry()}>
              Повторить
            </button>
          </div>
        </div>
      ) : null}

      <p className="small modal-note welcome-note">
        <Icon name="shield-check" /> Модели работают офлайн. Для видео приложение
        один раз подготовит локальный FFmpeg.
      </p>
    </Modal>
  );
}
