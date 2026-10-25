import type { ComponentProps } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { QueueFile } from "../types";
import { ExportScreen } from "./ExportScreen";

const completed: QueueFile[] = [
  {
    id: "a",
    path: "/tmp/interview.m4a",
    name: "Интервью с Анной Петровой.m4a",
    extension: "M4A",
    size_bytes: 38_600_000,
    status: "completed",
    progress: 100,
    processed_secs: 2538,
    duration_secs: 2538,
    segments: [{ start: 3, end: 8, text: "Текст интервью." }],
    saved_path: "/tmp/interview.md",
    error: null,
  },
  {
    id: "b",
    path: "/tmp/lecture.mp4",
    name: "Лекция 11. Компиляторы.mp4",
    extension: "MP4",
    size_bytes: 1_200_000_000,
    status: "completed",
    progress: 100,
    processed_secs: 4684,
    duration_secs: 4684,
    segments: [{ start: 3, end: 8, text: "Текст лекции." }],
    saved_path: "/tmp/lecture.md",
    error: null,
  },
];

function renderScreen(overrides: Partial<ComponentProps<typeof ExportScreen>> = {}) {
  const props: ComponentProps<typeof ExportScreen> = {
    completed,
    outputDir: "/Users/test/Documents/Transaudio",
    initialFormat: "md",
    onChooseFolder: vi.fn(),
    onOpenFolder: vi.fn(),
    onExport: vi.fn(async () => {}),
    ...overrides,
  };
  render(<ExportScreen {...props} />);
  return props;
}

describe("ExportScreen", () => {
  it("shows a calm empty state without completed transcripts", () => {
    renderScreen({ completed: [] });
    expect(screen.getByRole("heading", { name: "Экспорт" })).toBeInTheDocument();
    expect(screen.getByText("Пока нет готовых расшифровок.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Экспортировать/ })).not.toBeInTheDocument();
  });

  it("exports all selected files in the chosen format", async () => {
    const user = userEvent.setup();
    const onExport = vi.fn(async () => {});
    renderScreen({ onExport });

    await user.click(screen.getByRole("radio", { name: /Субтитры SRT/ }));
    await user.click(screen.getByRole("button", { name: "Экспортировать 2 файла" }));

    expect(onExport).toHaveBeenCalledWith(["a", "b"], "srt");
  });

  it("honours unchecked files", async () => {
    const user = userEvent.setup();
    const onExport = vi.fn(async () => {});
    renderScreen({ onExport });

    await user.click(screen.getByRole("checkbox", { name: /Интервью с Анной Петровой/ }));
    await user.click(screen.getByRole("button", { name: "Экспортировать 1 файл" }));

    expect(onExport).toHaveBeenCalledWith(["b"], "md");
  });

  it("shows the output folder and opens it", async () => {
    const user = userEvent.setup();
    const onOpenFolder = vi.fn();
    renderScreen({ onOpenFolder });

    await user.click(screen.getByRole("button", { name: /Открыть папку/ }));
    expect(onOpenFolder).toHaveBeenCalled();
    expect(screen.getByText("/Users/test/Documents/Transaudio")).toBeInTheDocument();
  });
});
