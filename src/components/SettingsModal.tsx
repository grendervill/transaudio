import type { ModelStatus, Settings } from "../types";
import { Icon } from "./Icon";
import { Modal } from "./Modal";

interface SettingsModalProps {
  open: boolean;
  settings: Settings;
  model: ModelStatus;
  modelName: string;
  onClose: () => void;
  onChooseFolder: () => void;
  onOpenFolder: () => void;
  onOpenModels: () => void;
  onRetryModel: () => void | Promise<void>;
}

function modelStateLabel(model: ModelStatus): string {
  switch (model.stage) {
    case "ready":
      return "Загружена";
    case "downloading":
      return `Загрузка ${Math.round(model.progress)}%`;
    case "loading":
      return "Запускается";
    case "not_installed":
      return "Не скачана";
    case "error":
      return "Ошибка";
    default:
      return "Не загружена";
  }
}

export function SettingsModal({
  open,
  settings,
  model,
  modelName,
  onClose,
  onChooseFolder,
  onOpenFolder,
  onOpenModels,
  onRetryModel,
}: SettingsModalProps) {
  return (
    <Modal open={open} onClose={onClose} titleId="settings-title" className="settings-modal">
      <header className="modal-head">
        <div>
          <p className="kicker">Настройки</p>
          <h2 id="settings-title">Рабочее пространство</h2>
        </div>
        <button className="icon-button" type="button" onClick={onClose} aria-label="Закрыть настройки">
          <Icon name="x" />
        </button>
      </header>

      <section className="settings-section">
        <p className="aside-label">Папка результатов</p>
        <div className="folder-picker">
          <span className="folder-symbol">
            <Icon name="folder" />
          </span>
          <span className="folder-copy">
            <strong>Transaudio</strong>
            <span className="mono">{settings.output_dir}</span>
          </span>
          <button className="button button-quiet" type="button" onClick={onChooseFolder}>
            Изменить
          </button>
        </div>
        <p className="small muted">
          Сюда сохраняются расшифровки, готовые для чтения и монтажа.
        </p>
      </section>

      <section className="settings-section">
        <div className="settings-model-head">
          <div>
            <p className="aside-label">Модель</p>
            <strong>{modelName}</strong>
            <p className="small muted">
              {model.stage === "ready"
                ? "Загружена · работает офлайн"
                : model.stage === "error"
                  ? model.error ?? "Ошибка загрузки"
                  : modelStateLabel(model)}
            </p>
          </div>
          <span
            className={`status-tag${
              model.stage === "error"
                ? " is-error"
                : model.stage === "ready"
                  ? " is-done"
                  : ""
            }`}
          >
            {modelStateLabel(model)}
          </span>
        </div>
        <p className="small settings-copy">
          Разрешение на загрузку моделей получено. Новые модели скачиваются по
          требованию, повторно приложение их не спрашивает. Модель стартует вместе с
          приложением.
        </p>
        <div className="settings-row">
          <span>Запускать модель с приложением</span>
          <span className="toggle is-on" aria-hidden="true">
            <i />
          </span>
        </div>
        <div className="settings-model-actions">
          <button className="button button-quiet" type="button" onClick={onOpenModels}>
            Сменить модель
          </button>
          {model.stage === "error" || model.stage === "not_installed" ? (
            <button
              className="button button-accent"
              type="button"
              onClick={() => void onRetryModel()}
            >
              {model.stage === "error" ? "Повторить загрузку" : "Скачать"}
            </button>
          ) : null}
        </div>
      </section>

      <footer className="modal-foot">
        <button className="text-button" type="button" onClick={onOpenFolder}>
          Открыть папку <Icon name="external-link" />
        </button>
        <button className="button button-accent" type="button" onClick={onClose}>
          Готово
        </button>
      </footer>
    </Modal>
  );
}
