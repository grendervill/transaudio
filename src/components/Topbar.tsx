import type { ModelStatus } from "../types";
import { Icon } from "./Icon";

interface TopbarProps {
  fileCount: number;
  model: ModelStatus;
  modelName: string;
  onOpenSettings: () => void;
}

function modelLabel(model: ModelStatus): string {
  switch (model.stage) {
    case "ready":
      return "на связи";
    case "downloading":
      return `загрузка ${Math.round(model.progress)}%`;
    case "loading":
      return "запускается";
    case "error":
      return "ошибка";
    default:
      return "не загружена";
  }
}

export function Topbar({ fileCount, model, modelName, onOpenSettings }: TopbarProps) {
  return (
    <header className="topbar">
      <div className="project">
        <span className="project-name">Рабочая очередь</span>
        <span className="project-meta">
          {fileCount === 0 ? "пока пусто" : `${fileCount} ${fileCount === 1 ? "запись" : "записи"}`}
        </span>
      </div>
      <div className="topbar-actions">
        <div className={`model-chip is-${model.stage}`} title="Локальная модель">
          <span className={`pulse ${model.stage}`} />
          <span className="model-chip-name">{modelName}</span>
          <span className="model-state">{modelLabel(model)}</span>
        </div>
        <button
          className="icon-button"
          type="button"
          onClick={onOpenSettings}
          aria-label="Настройки"
        >
          <Icon name="settings" />
        </button>
      </div>
    </header>
  );
}
