import type { ModelCard, ModelStatus } from "../types";
import { Modal } from "./Modal";
import { ModelPicker } from "./ModelPicker";

interface ModelPickerModalProps {
  open: boolean;
  models: ModelCard[];
  status: ModelStatus;
  canaryLanguage: string;
  busy: boolean;
  onClose: () => void;
  onSelect: (id: string) => void;
  onDownload: (id: string) => void;
  onDelete: (id: string) => void;
  onLanguage: (language: string) => void;
}

export function ModelPickerModal({
  open,
  models,
  status,
  canaryLanguage,
  busy,
  onClose,
  onSelect,
  onDownload,
  onDelete,
  onLanguage,
}: ModelPickerModalProps) {
  return (
    <Modal open={open} onClose={onClose} titleId="model-picker-title" className="model-modal">
      <header className="modal-head">
        <div>
          <p className="kicker">Локальные модели</p>
          <h2 id="model-picker-title">Выберите модель</h2>
          <p className="modal-lead">
            Скорость и качество — из бенчмарков Handy. Модели скачиваются по требованию
            и работают на этом компьютере.
          </p>
        </div>
        <button className="icon-button" type="button" onClick={onClose} aria-label="Закрыть">
          <svg className="icon" aria-hidden="true">
            <use href="/icons/lucide.svg#x" />
          </svg>
        </button>
      </header>
      <ModelPicker
        models={models}
        status={status}
        canaryLanguage={canaryLanguage}
        busy={busy}
        onSelect={onSelect}
        onDownload={onDownload}
        onDelete={onDelete}
        onLanguage={onLanguage}
      />
    </Modal>
  );
}
