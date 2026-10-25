import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { ModelStatus, Settings } from "../types";
import { SettingsModal } from "./SettingsModal";

const settings: Settings = {
  output_dir: "/Users/test/Documents/Transaudio",
  model_consent: true,
  export_format: "md",
};

const ready: ModelStatus = {
  stage: "ready",
  progress: 100,
  downloaded_bytes: null,
  total_bytes: null,
  error: null,
};

function renderModal(overrides: Partial<React.ComponentProps<typeof SettingsModal>> = {}) {
  const props: React.ComponentProps<typeof SettingsModal> = {
    open: true,
    settings,
    model: ready,
    onClose: vi.fn(),
    onChooseFolder: vi.fn(),
    onOpenFolder: vi.fn(),
    onAllowModel: vi.fn(async () => {}),
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

  it("says that permission was granted once", () => {
    renderModal();
    expect(screen.getByText(/Повторно разрешение не запрашивается/i)).toBeInTheDocument();
    expect(screen.getAllByText(/Загружена/).length).toBeGreaterThan(0);
  });

  it("offers retry if the model failed", async () => {
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
});
