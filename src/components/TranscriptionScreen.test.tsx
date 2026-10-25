import type { ComponentProps } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { ModelStatus, QueueFile, QueueState } from "../types";
import { TranscriptionScreen } from "./TranscriptionScreen";

const model: ModelStatus = {
  stage: "ready",
  progress: 100,
  downloaded_bytes: null,
  total_bytes: null,
  error: null,
};

const files: QueueFile[] = [
  {
    id: "one",
    path: "/tmp/interview.m4a",
    name: "Интервью с Анной Петровой.m4a",
    extension: "M4A",
    size_bytes: 38_600_000,
    status: "completed",
    progress: 100,
    processed_secs: 2538,
    duration_secs: 2538,
    segments: [{ start: 3, end: 8, text: "Если смотреть на архив, первое, что замечаешь — как меняется способ слушать." }],
    saved_path: "/tmp/interview.md",
    error: null,
  },
  {
    id: "two",
    path: "/tmp/lecture.mp4",
    name: "Лекция 11. Компиляторы.mp4",
    extension: "MP4",
    size_bytes: 1_200_000_000,
    status: "processing",
    progress: 64,
    processed_secs: 3130,
    duration_secs: 4684,
    segments: [
      { start: 3, end: 8, text: "Сегодня мы разберём, как исходный текст превращается в то, что процессор умеет исполнять." },
      { start: 41, end: 50, text: "Компилятор — это не один шаг, а цепочка превращений." },
    ],
    saved_path: null,
    error: null,
  },
  {
    id: "three",
    path: "/tmp/podcast.wav",
    name: "Подкаст, выпуск 07.wav",
    extension: "WAV",
    size_bytes: 498_000_000,
    status: "queued",
    progress: 0,
    processed_secs: null,
    duration_secs: 3393,
    segments: [],
    saved_path: null,
    error: null,
  },
];

const queue: QueueState = {
  status: "running",
  overall_progress: 41,
  current_id: "two",
  current_index: 1,
  total: 3,
  completed: 1,
  failed: 0,
};

function renderScreen(overrides: Partial<ComponentProps<typeof TranscriptionScreen>> = {}) {
  const props: React.ComponentProps<typeof TranscriptionScreen> = {
    files,
    queue,
    model,
    onPause: vi.fn(),
    onNotify: vi.fn(),
    onOpenSettings: vi.fn(),
    ...overrides,
  };
  render(<TranscriptionScreen {...props} />);
  return props;
}

describe("TranscriptionScreen", () => {
  it("shows the current file with a progress bar and live segments", () => {
    renderScreen();

    expect(screen.getByRole("heading", { name: "Лекция 11. Компиляторы" })).toBeInTheDocument();
    const progress = screen.getByRole("progressbar", { name: "Прогресс текущего файла" });
    expect(progress).toHaveAttribute("aria-valuenow", "64");
    expect(screen.getByText(/52:10 \/ 1:18:04/)).toBeInTheDocument();
    expect(screen.getByText(/Компилятор — это не один шаг/)).toBeInTheDocument();
  });

  it("keeps completed files compact and expands them on demand", async () => {
    const user = userEvent.setup();
    renderScreen();

    const summary = screen.getByRole("button", {
      name: /Интервью с Анной Петровой/,
    });
    expect(summary).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText(/меняется способ слушать/)).not.toBeInTheDocument();

    await user.click(summary);
    expect(summary).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText(/меняется способ слушать/)).toBeInTheDocument();
  });

  it("shows queued files in the sidebar without opening them", () => {
    renderScreen();
    expect(screen.getByText("Подкаст, выпуск 07.wav")).toBeInTheDocument();
    expect(screen.queryByText(/Подкаст.*текст/)).not.toBeInTheDocument();
  });

  it("shows an empty state when nothing has been added", () => {
    renderScreen({ files: [], queue: { ...queue, current_id: null, status: "idle", total: 0 } });
    expect(screen.getByText("Здесь появится расшифровка.")).toBeInTheDocument();
  });
  it("opens the most recent transcript when the queue is finished", () => {
    const doneFiles = files.slice(0, 2).map((file) => ({
      ...file,
      status: "completed" as const,
      progress: 100,
    }));
    renderScreen({
      files: doneFiles,
      queue: {
        ...queue,
        status: "done",
        overall_progress: 100,
        current_id: null,
        current_index: 2,
        total: 2,
        completed: 2,
      },
    });

    expect(
      screen.getByText(/Компилятор — это не один шаг/),
    ).toBeInTheDocument();
  });

  it("shows an intentional paused state instead of an empty transcript", () => {
    const pausedFiles = files.map((file) =>
      file.id === "two" ? { ...file, status: "paused" as const, segments: [] } : file,
    );
    renderScreen({
      files: pausedFiles,
      queue: { ...queue, status: "paused" },
    });

    expect(screen.getByText("Расшифровка на паузе.")).toBeInTheDocument();
    expect(screen.getAllByText(/пауза/i).length).toBeGreaterThan(0);
  });

  it("shows the reason when a file fails", () => {
    const failed = files.map((item) =>
      item.id === "two"
        ? {
            ...item,
            status: "error" as const,
            segments: [],
            error: "В файле нет аудиодорожки",
          }
        : item,
    );
    renderScreen({ files: failed, queue: { ...queue, status: "done", current_id: null } });

    expect(screen.getByText("В файле нет аудиодорожки")).toBeInTheDocument();
  });

});
