import type { ComponentProps } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { ModelStatus, Settings } from "../types";
import { SettingsModal } from "./SettingsModal";

const settings: Settings = {
  output_dir: "/Users/test/Documents/Transaudio",
  model_consent: true,
  export_format: "md",
  model_id: "parakeet-tdt-0.6b-v3",
  canary_language: "ru",
  diarization: false,
  diarization_scenario: "interview",
};

const ready: ModelStatus = {
  model_id: "parakeet-tdt-0.6b-v3",
  stage: "ready",
  progress: 100,
  downloaded_bytes: null,
  total_bytes: null,
  error: null,
};

function renderModal(overrides: Partial<ComponentProps<typeof SettingsModal>> = {}) {
  const props: ComponentProps<typeof SettingsModal> = {
    open: true,
    settings,
    model: ready,
    modelName: "Parakeet TDT 0.6B v3",
    onClose: vi.fn(),
    onChooseFolder: vi.fn(),
    onOpenFolder: vi.fn(),
    onOpenModels: vi.fn(),
    onRetryModel: vi.fn(async () => {}),
    ...overrides,
  };
  render(<SettingsModal {...props} />);
  return props;
}

describe("SettingsModal", () => {
  it("shows the working folder and lets the user change it", async () => {
    const user = userEvent.setup();
    const onChooseFolder = vi.fn();
    renderModal({ onChooseFolder });
    expect(screen.getByText("/Users/test/Documents/Transaudio")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Изменить" }));
    expect(onChooseFolder).toHaveBeenCalled();
  });

  it("shows the selected model and opens the picker", async () => {
    const user = userEvent.setup();
    const onOpenModels = vi.fn();
    renderModal({ onOpenModels });

    expect(screen.getByText("Parakeet TDT 0.6B v3")).toBeInTheDocument();
    expect(screen.getAllByText(/Загружена/).length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: "Сменить модель" }));
    expect(onOpenModels).toHaveBeenCalled();
  });

  it("offers a retry after a model failure", async () => {
    const user = userEvent.setup();
    const onRetryModel = vi.fn(async () => {});
    renderModal({
      model: {
        ...ready,
        stage: "error",
        error: "Модель повреждена",
      },
      onRetryModel,
    });
    await user.click(screen.getByRole("button", { name: "Повторить загрузку" }));
    expect(onRetryModel).toHaveBeenCalled();
  });

  it("offers a download when the selected model is missing", () => {
    renderModal({
      model: {
        ...ready,
        stage: "not_installed",
        progress: 0,
        error: null,
      },
    });
    expect(screen.getByRole("button", { name: "Скачать" })).toBeInTheDocument();
  });
});
