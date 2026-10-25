import { useEffect, useState } from "react";

import { formatDuration } from "../lib/format";
import type { ExportFormat, QueueFile } from "../types";
import { Icon } from "./Icon";

interface ExportScreenProps {
  completed: QueueFile[];
  outputDir: string;
  initialFormat: ExportFormat;
  onChooseFolder: () => void;
  onOpenFolder: () => void;
  onExport: (ids: string[], format: ExportFormat) => Promise<void>;
}

const FORMATS: Array<{ value: ExportFormat; title: string; detail: string }> = [
  { value: "md", title: "Markdown", detail: ".md · текст с таймкодами" },
  { value: "srt", title: "Субтитры SRT", detail: ".srt · для монтажа и плееров" },
  { value: "txt", title: "Простой текст", detail: ".txt · без разметки" },
];

function fileWord(count: number): string {
  if (count === 1) return "файл";
  if (count >= 2 && count <= 4) return "файла";
  return "файлов";
}

export function ExportScreen({
  completed,
  outputDir,
  initialFormat,
  onChooseFolder,
  onOpenFolder,
  onExport,
}: ExportScreenProps) {
  const [selected, setSelected] = useState<Set<string>>(
    () => new Set(completed.map((file) => file.id)),
  );
  const [format, setFormat] = useState<ExportFormat>(initialFormat);
  const [busy, setBusy] = useState(false);
  const completedKey = completed.map((file) => file.id).join("|");

  useEffect(() => {
    setSelected(new Set(completedKey ? completedKey.split("|") : []));
  }, [completedKey]);

  const chosen = completed.filter((file) => selected.has(file.id));
  const allSelected = completed.length > 0 && chosen.length === completed.length;

  function toggle(id: string) {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  async function exportNow() {
    if (!chosen.length || busy) return;
    setBusy(true);
    try {
      await onExport(
        chosen.map((file) => file.id),
        format,
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="screen export-screen" aria-label="Экспорт">
      <div className="export-head">
        <div>
          <p className="kicker">Выход из проекта</p>
          <h1>Экспорт</h1>
          <p className="lead">
            Выберите готовые расшифровки и формат. Markdown-копии уже сохраняются автоматически.
          </p>
        </div>
        <span className="export-count mono">
          {completed.length} / {completed.length} готовы
        </span>
      </div>

      <div className="export-layout">
        <section className="export-list-card" aria-label="Готовые расшифровки">
          <div className="section-head">
            <h2>Расшифровки</h2>
            {completed.length > 0 ? (
              <label className="check-all">
                <input
                  type="checkbox"
                  checked={allSelected}
                  onChange={() =>
                    setSelected(
                      allSelected ? new Set() : new Set(completed.map((file) => file.id)),
                    )
                  }
                />
                <span>Выбрать все</span>
              </label>
            ) : null}
          </div>

          {completed.length === 0 ? (
            <div className="export-empty">
              <Icon name="file-text" />
              <strong>Пока нет готовых расшифровок.</strong>
              <span>Они появятся здесь после завершения очереди.</span>
            </div>
          ) : (
            <div className="export-rows">
              {completed.map((file) => (
                <label key={file.id} className="export-row">
                  <input
                    type="checkbox"
                    checked={selected.has(file.id)}
                    onChange={() => toggle(file.id)}
                    aria-label={`Выбрать ${file.name}`}
                  />
                  <span className="file-kind">
                    <Icon name="file-text" />
                  </span>
                  <span className="export-copy">
                    <strong>{file.name.replace(/\.[^.]+$/, "")}</strong>
                    <span>
                      .md ·{" "}
                      {file.duration_secs ? formatDuration(file.duration_secs) : "готово"}
                      {file.saved_path ? " · сохранено" : ""}
                    </span>
                  </span>
                  <span className="status-tag is-done">Готово</span>
                </label>
              ))}
            </div>
          )}
        </section>

        <aside className="export-aside">
          <section className="aside-block">
            <p className="aside-label">Куда сохранить</p>
            <div className="folder-line">
              <Icon name="folder" />
              <span className="mono path">{outputDir}</span>
            </div>
            <div className="aside-links">
              <button className="text-button" type="button" onClick={onChooseFolder}>
                Изменить папку <Icon name="arrow-up-right" />
              </button>
              <button className="text-button" type="button" onClick={onOpenFolder}>
                Открыть папку <Icon name="external-link" />
              </button>
            </div>
          </section>

          <section className="aside-block">
            <p className="aside-label">Формат экспорта</p>
            <div className="format-list" role="radiogroup" aria-label="Формат экспорта">
              {FORMATS.map((option) => (
                <label
                  key={option.value}
                  className={`format-option${format === option.value ? " is-selected" : ""}`}
                >
                  <input
                    type="radio"
                    name="export-format"
                    value={option.value}
                    checked={format === option.value}
                    onChange={() => setFormat(option.value)}
                  />
                  <span>
                    <strong>{option.title}</strong>
                    <small>{option.detail}</small>
                  </span>
                  <span className="radio-dot" aria-hidden="true" />
                </label>
              ))}
            </div>
          </section>

          {completed.length > 0 ? (
            <section className="export-summary">
              <div>
                <span>К записи</span>
                <strong>
                  {chosen.length} {fileWord(chosen.length)}
                </strong>
              </div>
              <button
                className="button button-accent export-button"
                type="button"
                onClick={exportNow}
                disabled={!chosen.length || busy}
              >
                <Icon name="download" />
                {busy
                  ? "Сохраняем…"
                  : `Экспортировать ${chosen.length} ${fileWord(chosen.length)}`}
              </button>
              <p className="small muted center">
                Оригинальные записи останутся на своих местах.
              </p>
            </section>
          ) : (
            <p className="small muted export-hint">
              Сначала завершите хотя бы одну расшифровку.
            </p>
          )}
        </aside>
      </div>
    </section>
  );
}
