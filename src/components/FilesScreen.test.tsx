import type { ComponentProps } from "react";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { ModelStatus, QueueFile } from "../types";
import { createFakeBackend } from "../test/fakeBackend";
import { FilesScreen } from "./FilesScreen";

const readyModel: ModelStatus = {
  stage: "ready",
  progress: 100,
  downloaded_bytes: null,
  total_bytes: null,
  error: null,
};

const blockedModel: ModelStatus = {
  ...readyModel,
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
    outputDir: "/Users/test/Documents/Transaudio",
    queueStatus: "idle",
    onAddPaths: vi.fn(),
    onRemoveFile: vi.fn(),
    onStart: vi.fn(),
    onChooseFolder: vi.fn(),
    onOpenSettings: vi.fn(),
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
    expect(screen.getByText(/модель не загружена/i)).toBeInTheDocument();
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
      outputDir: "/tmp/out",
      queueStatus: "idle" as const,
      onAddPaths: vi.fn(),
      onRemoveFile: vi.fn(),
      onStart: vi.fn(),
      onChooseFolder: vi.fn(),
      onOpenSettings: vi.fn(),
      onNotify: vi.fn(),
    };
    const { rerender } = render(<FilesScreen {...props} />);
    await waitFor(() => expect(onDragDrop).toHaveBeenCalledTimes(1));

    rerender(<FilesScreen {...props} onNotify={vi.fn()} />);

    await waitFor(() => expect(onDragDrop).toHaveBeenCalledTimes(1));
  });

});
