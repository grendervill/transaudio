import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import App from "./App";
import { createFakeBackend, defaultBootstrap } from "./test/fakeBackend";
import type { FileUpdate, ModelStatus } from "./types";

function renderApp() {
  return render(<App backend={createFakeBackend()} />);
}

describe("app shell", () => {
  it("renders the project navigation after bootstrap", async () => {
    renderApp();

    expect(
      await screen.findByRole("navigation", { name: "Основная навигация" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Файлы/ })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(
      screen.getByRole("button", { name: /Транскрипция/ }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Экспорт/ })).toBeInTheDocument();
    expect(screen.getByText("Рабочая очередь")).toBeInTheDocument();
    expect(
      screen.getByRole("contentinfo", { name: "Общий прогресс очереди" }),
    ).toBeInTheDocument();
  });

  it("switches between the three workspaces", async () => {
    const user = userEvent.setup();
    renderApp();
    await screen.findByRole("navigation", { name: "Основная навигация" });

    await user.click(screen.getByRole("button", { name: /Транскрипция/ }));
    expect(
      screen.getByRole("region", { name: "Транскрипция" }),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Экспорт/ }));
    expect(screen.getByRole("region", { name: "Экспорт" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Файлы/ }));
    await waitFor(() => {
      expect(
        screen.getByRole("region", { name: "Файлы" }),
      ).toBeInTheDocument();
    });
  });
  it("does not ask for permission again when consent is already persisted", async () => {
    render(
      <App
        backend={createFakeBackend({
          bootstrap: async () => ({
            ...defaultBootstrap,
            settings: {
              output_dir: "/tmp/out",
              model_consent: true,
              export_format: "md",
              model_id: "parakeet-tdt-0.6b-v3",
              canary_language: "ru",
              diarization: false,
              diarization_scenario: "interview",
              diarization_speakers: 0,
            },
            model: {
              model_id: "parakeet-tdt-0.6b-v3",
              stage: "consent_required",
              progress: 0,
              downloaded_bytes: null,
              total_bytes: null,
              error: null,
            },
            queue: {
              status: "idle",
              overall_progress: 0,
              current_id: null,
              current_index: 0,
              total: 0,
              completed: 0,
              failed: 0,
            },
          }),
        })}
      />,
    );

    await screen.findByRole("navigation", { name: "Основная навигация" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("shows the model picker on first run and downloads the chosen model", async () => {
    const user = userEvent.setup();
    const downloadModel = vi.fn(
      () => new Promise<ModelStatus>(() => {}),
    );
    render(
      <App
        backend={createFakeBackend({
          bootstrap: async () => ({
            ...defaultBootstrap,
            settings: {
              ...defaultBootstrap.settings,
              model_consent: false,
            },
            model: {
              ...defaultBootstrap.model,
              stage: "consent_required",
              progress: 0,
            },
          }),
          downloadModel,
        })}
      />,
    );

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("GigaAM v3 E2E-RNNT")).toBeInTheDocument();

    const card = within(dialog)
      .getByText("GigaAM v3 E2E-RNNT")
      .closest("article");
    await user.click(card?.querySelector("button") as HTMLButtonElement);

    expect(downloadModel).toHaveBeenCalledWith("gigaam-v3-e2e-rnnt", undefined);
  });

  it("keeps a failed model download visible", async () => {
    const user = userEvent.setup();
    render(
      <App
        backend={createFakeBackend({
          bootstrap: async () => ({
            ...defaultBootstrap,
            settings: {
              ...defaultBootstrap.settings,
              model_consent: false,
            },
            model: {
              ...defaultBootstrap.model,
              stage: "consent_required",
              progress: 0,
            },
          }),
          downloadModel: async () => {
            throw new Error("Нет соединения с сервером");
          },
        })}
      />,
    );

    const dialog = await screen.findByRole("dialog");
    const card = within(dialog)
      .getByText("GigaAM v3 E2E-RNNT")
      .closest("article");
    await user.click(card?.querySelector("button") as HTMLButtonElement);

    expect(
      within(dialog).getAllByText("Нет соединения с сервером").length,
    ).toBeGreaterThan(0);
    expect(
      within(dialog).getAllByRole("button", { name: "Повторить" }).length,
    ).toBeGreaterThan(0);
  });

  it("opens the model error on startup when a previous download failed", async () => {
    render(
      <App
        backend={createFakeBackend({
          bootstrap: async () => ({
            ...defaultBootstrap,
            settings: {
              output_dir: "/tmp/out",
              model_consent: true,
              export_format: "md",
              model_id: "parakeet-tdt-0.6b-v3",
              canary_language: "ru",
              diarization: false,
              diarization_scenario: "interview",
              diarization_speakers: 0,
            },
            model: {
              model_id: "parakeet-tdt-0.6b-v3",
              stage: "error",
              progress: 0,
              downloaded_bytes: null,
              total_bytes: null,
              error: "Не удалось скачать модель",
            },
            queue: {
              status: "idle",
              overall_progress: 0,
              current_id: null,
              current_index: 0,
              total: 0,
              completed: 0,
              failed: 0,
            },
          }),
        })}
      />,
    );

    const dialog = await screen.findByRole("dialog");
    expect(
      within(dialog).getAllByText("Не удалось скачать модель").length,
    ).toBeGreaterThan(0);
  });
  it("starts only new files and opens a finished transcript on click", async () => {
    const user = userEvent.setup();
    let pushUpdate: (update: FileUpdate) => void = () => {};
    const pickFiles = vi
      .fn()
      .mockResolvedValueOnce(["/tmp/a.mp3"])
      .mockResolvedValueOnce(["/tmp/b.mp3"]);
    const startTranscription = vi.fn(async () => ({
      status: "running" as const,
      overall_progress: 0,
      current_id: null,
      current_index: 0,
      total: 1,
      completed: 0,
      failed: 0,
      message: null,
    }));
    render(
      <App
        backend={createFakeBackend({
          pickFiles,
          startTranscription,
          onFileUpdate: async (callback) => {
            pushUpdate = callback;
            return () => {};
          },
        })}
      />,
    );
    await screen.findByRole("navigation", { name: "Основная навигация" });

    await user.click(screen.getByRole("button", { name: "Добавить файлы" }));
    pushUpdate({
      id: "file-/tmp/a.mp3",
      status: "completed",
      progress: 100,
      processed_secs: 9,
      duration_secs: 9,
      new_segments: [{ start: 0, end: 4, text: "Первый готовый текст." }],
      saved_path: "/tmp/a.md",
      error: null,
    });
    await screen.findByText("Готово");

    await user.click(screen.getByRole("button", { name: "Добавить файлы" }));
    await screen.findByText("b.mp3");

    await user.click(screen.getByRole("button", { name: "Начать" }));
    expect(startTranscription).toHaveBeenCalledWith([
      { id: "file-/tmp/b.mp3", path: "/tmp/b.mp3", name: "b.mp3" },
    ]);

    await user.click(screen.getByRole("button", { name: /Файлы/ }));
    await user.click(
      screen.getByRole("button", { name: /Открыть расшифровку a.mp3/ }),
    );
    expect(screen.getByText("Первый готовый текст.")).toBeInTheDocument();
  });

});
