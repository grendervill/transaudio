import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { ModelStatus } from "../types";
import { OnboardingModal } from "./OnboardingModal";

const consentRequired: ModelStatus = {
  stage: "consent_required",
  progress: 0,
  downloaded_bytes: null,
  total_bytes: null,
  error: null,
};

function renderModal(
  model: ModelStatus = consentRequired,
  overrides: Partial<React.ComponentProps<typeof OnboardingModal>> = {},
) {
  const props: React.ComponentProps<typeof OnboardingModal> = {
    open: true,
    model,
    onAllow: vi.fn(async () => {}),
    onRetry: vi.fn(async () => {}),
    onDismiss: vi.fn(),
    ...overrides,
  };
  render(<OnboardingModal {...props} />);
  return props;
}

describe("OnboardingModal", () => {
  it("asks once for permission and explains the local model", () => {
    renderModal();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText(/640 МБ/)).toBeInTheDocument();
    expect(screen.getByText(/аудио.*остаются/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Разрешить загрузку" })).toBeInTheDocument();
  });

  it("starts the download only after explicit permission", async () => {
    const user = userEvent.setup();
    const onAllow = vi.fn(async () => {});
    renderModal(consentRequired, { onAllow });

    await user.click(screen.getByRole("button", { name: "Разрешить загрузку" }));
    expect(onAllow).toHaveBeenCalledOnce();
  });

  it("shows download progress while the model is being fetched", () => {
    renderModal({
      stage: "downloading",
      progress: 42,
      downloaded_bytes: 268_000_000,
      total_bytes: 640_000_000,
      error: null,
    });
    expect(screen.getByRole("progressbar", { name: "Загрузка модели" })).toHaveAttribute(
      "aria-valuenow",
      "42",
    );
    expect(screen.queryByRole("button", { name: "Разрешить загрузку" })).not.toBeInTheDocument();
  });

  it("offers a retry after a failed download", async () => {
    const user = userEvent.setup();
    const onRetry = vi.fn(async () => {});
    renderModal(
      {
        stage: "error",
        progress: 0,
        downloaded_bytes: null,
        total_bytes: null,
        error: "Нет соединения",
      },
      { onRetry },
    );
    expect(screen.getByText("Нет соединения")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Повторить" }));
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it("can be dismissed for this session", async () => {
    const user = userEvent.setup();
    const onDismiss = vi.fn();
    renderModal(consentRequired, { onDismiss });
    await user.click(screen.getByRole("button", { name: "Не сейчас" }));
    expect(onDismiss).toHaveBeenCalled();
  });
});
