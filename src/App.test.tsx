import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import App from "./App";
import { createFakeBackend } from "./test/fakeBackend";

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
            settings: {
              output_dir: "/tmp/out",
              model_consent: true,
              export_format: "md",
            },
            model: {
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

});
