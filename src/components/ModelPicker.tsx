import { useMemo, useState } from "react";

import { formatBytes } from "../lib/format";
import {
  isAsian,
  languageName,
  languageSummary,
  qualityScore,
  recommendationLabel,
  speedScore,
  supportsLanguage,
  werSummary,
} from "../lib/modelMeta";
import type { ModelCard, ModelStatus } from "../types";
import { Icon } from "./Icon";

type Filter = "all" | "ru" | "en" | "asian";

interface ModelPickerProps {
  models: ModelCard[];
  status: ModelStatus;
  canaryLanguage: string;
  busy: boolean;
  onSelect: (id: string) => void;
  onDownload: (id: string) => void;
  onDelete: (id: string) => void;
  onLanguage: (language: string) => void;
}

function Bars({ label, value }: { label: string; value: number | null }) {
  return (
    <div className="model-bar">
      <span>{label}</span>
      <span className="model-bar-track" aria-label={`${label}: ${value ?? 0} из 5`}>
        {[1, 2, 3, 4, 5].map((step) => (
          <i key={step} className={value !== null && step <= value ? "is-on" : ""} />
        ))}
      </span>
    </div>
  );
}

export function ModelPicker({
  models,
  status,
  canaryLanguage,
  busy,
  onSelect,
  onDownload,
  onDelete,
  onLanguage,
}: ModelPickerProps) {
  const [filter, setFilter] = useState<Filter>("all");

  const visible = useMemo(() => {
    if (filter === "all") return models;
    if (filter === "asian") return models.filter(isAsian);
    return models.filter((model) => supportsLanguage(model, filter));
  }, [models, filter]);

  const selectedModel = models.find((model) => model.selected);

  return (
    <div className="model-picker">
      <div className="model-filters" role="tablist" aria-label="Фильтр моделей">
        {(
          [
            ["all", "Все"],
            ["ru", "Русский"],
            ["en", "Английский"],
            ["asian", "Азиатские"],
          ] as Array<[Filter, string]>
        ).map(([value, label]) => (
          <button
            key={value}
            type="button"
            role="tab"
            aria-selected={filter === value}
            className={`model-filter${filter === value ? " is-active" : ""}`}
            onClick={() => setFilter(value)}
          >
            {label}
          </button>
        ))}
      </div>

      <div className="model-grid">
        {visible.map((card) => {
          const isSelected = card.selected;
          const downloading =
            isSelected && status.stage === "downloading" && status.model_id === card.id;
          const loading = isSelected && status.stage === "loading" && status.model_id === card.id;
          const recommended = recommendationLabel(card);
          const progress = downloading
            ? status.total_bytes
              ? Math.min(99, (status.downloaded_bytes ?? 0) / status.total_bytes * 100)
              : status.progress
            : 0;

          return (
            <article
              key={card.id}
              className={`model-card${isSelected ? " is-selected" : ""}`}
            >
              <header className="model-card-head">
                <div className="model-card-title">
                  <h3>{card.name}</h3>
                  <span className="model-family">{card.family}</span>
                </div>
                {recommended ? (
                  <span className="model-tag is-recommended">{recommended}</span>
                ) : card.badge ? (
                  <span className="model-tag">{card.badge}</span>
                ) : null}
              </header>

              <p className="model-description">{card.description}</p>
              <p className="model-languages">{languageSummary(card)}</p>

              <div className="model-bars">
                <Bars label="Скорость" value={speedScore(card.speed_x)} />
                <Bars label="Качество" value={qualityScore(card)} />
              </div>
              <p className="model-wer mono">
                {werSummary(card) ?? "WER нет в каталоге"}
              </p>

              {card.needs_language && isSelected ? (
                <label className="model-language">
                  <span>Язык записи</span>
                  <select
                    value={canaryLanguage}
                    onChange={(event) => onLanguage(event.target.value)}
                  >
                    {card.languages.map((code) => (
                      <option key={code} value={code}>
                        {languageName(code)}
                      </option>
                    ))}
                  </select>
                </label>
              ) : null}

              {downloading ? (
                <div className="model-download">
                  <div
                    className="model-download-track"
                    role="progressbar"
                    aria-label="Загрузка модели"
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuenow={Math.round(progress)}
                  >
                    <i style={{ width: `${progress}%` }} />
                  </div>
                  <span className="mono">
                    {status.total_bytes
                      ? `${formatBytes(status.downloaded_bytes ?? 0)} / ${formatBytes(status.total_bytes)}`
                      : `${Math.round(progress)}%`}
                  </span>
                </div>
              ) : null}

              {isSelected && status.stage === "error" && status.model_id === card.id ? (
                <p className="model-card-error">{status.error}</p>
              ) : null}

              <footer className="model-card-foot">
                <span className="model-size mono">
                  {formatBytes(card.size_bytes)} · {card.license}
                </span>
                <div className="model-actions">
                  {card.installed && !isSelected ? (
                    <button
                      className="text-button model-delete"
                      type="button"
                      disabled={busy}
                      onClick={() => onDelete(card.id)}
                    >
                      Удалить
                    </button>
                  ) : null}
                  {isSelected ? (
                    downloading || loading ? (
                      <span className="model-state mono">
                        {loading ? "запускается…" : `${Math.round(progress)}%`}
                      </span>
                    ) : status.stage === "ready" ? (
                      <span className="model-state is-ready">
                        <Icon name="check" /> Выбрана
                      </span>
                    ) : (
                      <button
                        className="button button-accent model-action"
                        type="button"
                        disabled={busy}
                        onClick={() => onDownload(card.id)}
                      >
                        {status.stage === "error" ? "Повторить" : "Скачать"}
                      </button>
                    )
                  ) : card.installed ? (
                    <button
                      className="button button-quiet model-action"
                      type="button"
                      disabled={busy}
                      onClick={() => onSelect(card.id)}
                    >
                      Выбрать
                    </button>
                  ) : (
                    <button
                      className="button button-accent model-action"
                      type="button"
                      disabled={busy}
                      onClick={() => onDownload(card.id)}
                    >
                      Скачать
                    </button>
                  )}
                </div>
              </footer>
            </article>
          );
        })}
      </div>

      {selectedModel ? (
        <p className="model-picker-foot">
          Выбрана: {selectedModel.name}. Модели скачиваются по требованию и работают
          офлайн.
        </p>
      ) : null}
    </div>
  );
}
