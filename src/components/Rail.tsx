import type { ModelStatus, Screen } from "../types";
import { Icon } from "./Icon";

interface RailProps {
  screen: Screen;
  model: ModelStatus;
  onNavigate: (screen: Screen) => void;
  onOpenSettings: () => void;
}

const NAV: Array<{ id: Screen; label: string; icon: string }> = [
  { id: "files", label: "Файлы", icon: "plus" },
  { id: "transcribe", label: "Транскрипция", icon: "audio-waveform" },
  { id: "export", label: "Экспорт", icon: "folder-output" },
];

function modelTone(model: ModelStatus): string {
  switch (model.stage) {
    case "ready":
      return "is-ready";
    case "error":
      return "is-error";
    default:
      return "is-waiting";
  }
}

export function Rail({ screen, model, onNavigate, onOpenSettings }: RailProps) {
  return (
    <aside className="rail">
      <a className="brand" href="#" onClick={(event) => event.preventDefault()} aria-label="Transaudio">
        <Icon name="audio-lines" />
      </a>
      <nav className="rail-nav" aria-label="Основная навигация">
        {NAV.map((item) => (
          <button
            key={item.id}
            type="button"
            className={`rail-link${screen === item.id ? " is-active" : ""}`}
            aria-current={screen === item.id ? "page" : undefined}
            onClick={() => onNavigate(item.id)}
          >
            <Icon name={item.icon} />
            <span>{item.label}</span>
          </button>
        ))}
      </nav>
      <div className="rail-bottom">
        <span className={`rail-led ${modelTone(model)}`} title="Статус модели" />
        <button
          type="button"
          className="rail-link rail-settings"
          aria-label="Настройки"
          onClick={onOpenSettings}
        >
          <Icon name="settings" />
          <span>Настройки</span>
        </button>
      </div>
    </aside>
  );
}
