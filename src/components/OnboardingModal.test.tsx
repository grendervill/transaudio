import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { ModelStatus } from "../types";
import { sampleModels } from "../test/fakeBackend";
import { OnboardingModal } from "./OnboardingModal";

const consentRequired: ModelStatus = {
  model_id: "parakeet-tdt-0.6b-v3",
  stage: "consent_required",
  progress: 0,
  downloaded_bytes: null,
  total_bytes: null,
  error: null,
};

function renderModal(
  status: ModelStatus = consentRequired,
  overrides: Partial<React.ComponentProps<typeof OnboardingModal>> = {},
) {
  const props: React.ComponentProps<typeof OnboardingModal> = {
    open: true,
    models: sampleModels,
    status,
    canaryLanguage: "ru",
    onSelect: vi.fn(),
    onDownload: vi.fn(),
    onLanguage: vi.fn(),
    onRetry: vi.fn(async () => {}),
    onDismiss: vi.fn(),
    ...overrides,
  };
  render(<OnboardingModal {...props} />);
  return props;
}

describe("OnboardingModal", () => {
  it("shows the model catalog and the recommendation badges", () => {
    renderModal();

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText("Parakeet TDT 0.6B v3")).toBeInTheDocument();
    expect(screen.getByText("GigaAM v3 E2E-RNNT")).toBeInTheDocument();
    expect(screen.getByText("Рекомендую · русский")).toBeInTheDocument();
    expect(screen.getByText(/Аудио и текст остаются/)).toBeInTheDocument();
  });

  it("downloads a chosen model", async () => {
    const user = userEvent.setup();
    const onDownload = vi.fn();
    renderModal(consentRequired, { onDownload });

    const card = screen.getByText("GigaAM v3 E2E-RNNT").closest("article");
    expect(card).not.toBeNull();
    const button = card?.querySelector("button");
    expect(button).not.toBeNull();
    await user.click(button as HTMLButtonElement);

    expect(onDownload).toHaveBeenCalledWith("gigaam-v3-e2e-rnnt");
  });

  it("shows download progress for the selected model", () => {
    renderModal({
      model_id: "parakeet-tdt-0.6b-v3",
      stage: "downloading",
      progress: 42,
      downloaded_bytes: 268_000_000,
      total_bytes: 739_508_576,
      error: null,
    });

    expect(screen.getByText(/268 МБ \/ 739,5 МБ/)).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "Загрузка модели" })).toBeInTheDocument();
  });

  it("offers a retry after a failed download", async () => {
    const user = userEvent.setup();
    const onRetry = vi.fn(async () => {});
    renderModal(
      {
        model_id: "parakeet-tdt-0.6b-v3",
        stage: "error",
        progress: 0,
        downloaded_bytes: null,
        total_bytes: null,
        error: "Нет соединения",
      },
      { onRetry },
    );

    expect(screen.getAllByText("Нет соединения").length).toBeGreaterThan(0);
    const retries = screen.getAllByRole("button", { name: "Повторить" });
    await user.click(retries[retries.length - 1]);
    expect(onRetry).toHaveBeenCalled();
  });

  it("can be dismissed for this session", async () => {
    const user = userEvent.setup();
    const onDismiss = vi.fn();
    renderModal(consentRequired, { onDismiss });
    await user.click(screen.getByRole("button", { name: "Не сейчас" }));
    expect(onDismiss).toHaveBeenCalled();
  });
});
