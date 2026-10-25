import type { ComponentProps } from "react";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { ModelStatus, QueueFile } from "../types";
import { createFakeBackend } from "../test/fakeBackend";
import { FilesScreen } from "./FilesScreen";

const readyModel: ModelStatus = {
  model_id: "parakeet-tdt-0.6b-v3",
  stage: "ready",
  progress: 100,
  downloaded_bytes: null,
  total_bytes: null,
  error: null,
};

const blockedModel: ModelStatus = {
  ...readyModel,
  model_id: "parakeet-tdt-0.6b-v3",
  stage: "consent_required",
  progress: 0,
};

const file: QueueFile = {
  id: "f1",
  path: "/tmp/lecture.mp4",
  name: "Лекция 11. Компиляторы.mp4",
  extension: "MP4",
  size_bytes: 1_200_000_000,
  status: "queued",
  progress: 0,
  processed_secs: null,
  duration_secs: 4684,
  segments: [],
  saved_path: null,
  error: null,
};

function renderScreen(overrides: Partial<ComponentProps<typeof FilesScreen>> = {}) {
  const props: React.ComponentProps<typeof FilesScreen> = {
    backend: createFakeBackend(),
    files: [],
    model: readyModel,
    modelName: "Parakeet TDT 0.6B v3",
    outputDir: "/Users/test/Documents/Transaudio",
    diarizationEnabled: false,
    diarizationScenario: "interview",
    diarizationSpeakers: 0,
    diarizationStatus: {
      stage: "not_downloaded" as const,
      progress: 0,
      error: null,
    },
    queueStatus: "idle",
    onAddPaths: vi.fn(),
    onRemoveFile: vi.fn(),
    onStart: vi.fn(),
    onSelectFile: vi.fn(),
    onClear: vi.fn(),
    onChooseFolder: vi.fn(),
    onOpenSettings: vi.fn(),
    onOpenModels: vi.fn(),
    onDiarizationChange: vi.fn(),
    ...overrides,
  };
  render(<FilesScreen {...props} />);
  return props;
}

describe("FilesScreen", () => {
  it("shows an inviting empty state and disables start without files", () => {
    renderScreen();
    expect(screen.getByRole("heading", { name: "Добавьте записи" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Начать" })).toBeDisabled();
    expect(screen.getByText("Перетащите аудио или видео")).toBeInTheDocument();
  });

  it("enables start when files and a ready model exist", () => {
    renderScreen({ files: [file] });
    expect(screen.getByRole("button", { name: "Начать" })).toBeEnabled();
    expect(screen.getByText("Лекция 11. Компиляторы.mp4")).toBeInTheDocument();
  });

  it("blocks start while the model is not ready", () => {
    renderScreen({ files: [file], model: blockedModel });
    expect(screen.getByRole("button", { name: "Начать" })).toBeDisabled();
    expect(screen.getByText(/не загружена/i)).toBeInTheDocument();
  });

  it("passes selected files from the native picker to the queue", async () => {
    const user = userEvent.setup();
    const pickFiles = vi.fn(async () => ["/tmp/new.mp3"]);
    const backend = createFakeBackend({ pickFiles });
    const props = renderScreen({ backend });

    await user.click(screen.getByRole("button", { name: "Выбрать файлы" }));

    expect(pickFiles).toHaveBeenCalled();
    expect(props.onAddPaths).toHaveBeenCalledWith(["/tmp/new.mp3"]);
  });

  it("removes an idle file and calls start", async () => {
    const user = userEvent.setup();
    const props = renderScreen({ files: [file] });

    await user.click(screen.getByRole("button", { name: "Убрать Лекция 11. Компиляторы.mp4" }));
    expect(props.onRemoveFile).toHaveBeenCalledWith("f1");

    await user.click(screen.getByRole("button", { name: "Начать" }));
    expect(props.onStart).toHaveBeenCalled();
  });

  it("does not allow queue changes while running", () => {
    renderScreen({ files: [file], queueStatus: "running" });
    expect(screen.getByRole("button", { name: /Выбрать файлы/ })).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Убрать Лекция 11. Компиляторы.mp4" }),
    ).toBeDisabled();
    expect(screen.getByRole("button", { name: "Начать" })).toBeDisabled();
  });
  it("subscribes to native file drops once, not on every parent render", async () => {
    const onDragDrop = vi.fn(async (_callback: (event: { type: string }) => void) => () => {});
    const backend = createFakeBackend({ onDragDrop });
    const props = {
      backend,
      files: [],
      model: readyModel,
      modelName: "Parakeet TDT 0.6B v3",
      outputDir: "/tmp/out",
      diarizationEnabled: false,
      diarizationScenario: "interview",
      diarizationSpeakers: 0,
      diarizationStatus: { stage: "not_downloaded" as const, progress: 0, error: null },
      queueStatus: "idle" as const,
      onAddPaths: vi.fn(),
      onRemoveFile: vi.fn(),
      onStart: vi.fn(),
      onSelectFile: vi.fn(),
      onClear: vi.fn(),
      onChooseFolder: vi.fn(),
      onOpenSettings: vi.fn(),
      onOpenModels: vi.fn(),
      onDiarizationChange: vi.fn(),
      onNotify: vi.fn(),
    };
    const { rerender } = render(<FilesScreen {...props} />);
    await waitFor(() => expect(onDragDrop).toHaveBeenCalledTimes(1));

    rerender(<FilesScreen {...props} onNotify={vi.fn()} />);

    await waitFor(() => expect(onDragDrop).toHaveBeenCalledTimes(1));
  });

  it("enables start again when a new file is added after a finished queue", () => {
    const completed = {
      ...file,
      status: "completed" as const,
      progress: 100,
      segments: [{ start: 0, end: 3, text: "Готовый текст." }],
    };
    const fresh = { ...file, id: "f2", name: "Новая запись.mp3", status: "queued" as const };

    renderScreen({ files: [completed], queueStatus: "done" });
    expect(
      screen.getByRole("button", { name: "Все файлы расшифрованы" }),
    ).toBeDisabled();

    renderScreen({ files: [completed, fresh], queueStatus: "done" });
    expect(screen.getByRole("button", { name: "Начать" })).toBeEnabled();
  });

  it("opens a file transcript when its row is clicked", async () => {
    const user = userEvent.setup();
    const props = renderScreen({ files: [file] });

    await user.click(
      screen.getByRole("button", { name: /Открыть расшифровку Лекция 11/ }),
    );
    expect(props.onSelectFile).toHaveBeenCalledWith("f1");
  });

  it("allows clearing a finished queue without offering a rerun", async () => {
    const user = userEvent.setup();
    const onClear = vi.fn();
    const completed = { ...file, status: "completed" as const, progress: 100 };
    renderScreen({ files: [completed], queueStatus: "done", onClear });

    expect(
      screen.getByRole("button", { name: "Все файлы расшифрованы" }),
    ).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Очистить" }));
    expect(onClear).toHaveBeenCalled();
  });

  it("shows the actual file error instead of only a status label", () => {
    renderScreen({
      files: [
        {
          ...file,
          status: "error",
          error: "FFmpeg не смог извлечь аудио",
        },
      ],
    });
    expect(screen.getByText("FFmpeg не смог извлечь аудио")).toBeInTheDocument();
  });

  it("keeps a failed file retryable", () => {
    renderScreen({
      files: [{ ...file, status: "error", error: "Не удалось" }],
      queueStatus: "error",
    });
    expect(screen.getByRole("button", { name: "Начать" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Очистить" })).toBeEnabled();
  });

  it("lets the user pin the exact number of speakers", async () => {
    const user = userEvent.setup();
    const onDiarizationChange = vi.fn();
    renderScreen({
      diarizationEnabled: true,
      diarizationSpeakers: 0,
      onDiarizationChange,
    });

    await user.selectOptions(
      screen.getByRole("combobox", { name: "Число спикеров" }),
      "3",
    );

    expect(onDiarizationChange).toHaveBeenCalledWith(true, "interview", 3);
  });

  it("switches diarization scenarios without losing the pinned count", async () => {
    const user = userEvent.setup();
    const onDiarizationChange = vi.fn();
    renderScreen({
      diarizationEnabled: true,
      diarizationScenario: "interview",
      diarizationSpeakers: 4,
      onDiarizationChange,
    });

    await user.selectOptions(
      screen.getByRole("combobox", { name: "Сценарий записи" }),
      "meeting",
    );

    expect(onDiarizationChange).toHaveBeenCalledWith(true, "meeting", 4);
  });

});
