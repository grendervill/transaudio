import { describe, expect, it } from "vitest";

import type { ModelStatus } from "../types";
import { reconcileModelStatus } from "./modelStatus";

function status(overrides: Partial<ModelStatus>): ModelStatus {
  return {
    model_id: "parakeet-tdt-0.6b-v3",
    stage: "consent_required",
    progress: 0,
    downloaded_bytes: null,
    total_bytes: null,
    error: null,
    ...overrides,
  };
}

describe("reconcileModelStatus", () => {
  it("accepts the first status", () => {
    expect(reconcileModelStatus(null, status({ stage: "downloading" })).stage).toBe(
      "downloading",
    );
  });

  it("ignores a stale consent_required after the user granted permission", () => {
    const current = status({ stage: "downloading", progress: 42 });
    const stale = reconcileModelStatus(current, status({ stage: "consent_required" }));

    expect(stale.stage).toBe("downloading");
    expect(stale.progress).toBe(42);
  });

  it("keeps a visible error even if a stale reply says consent is required", () => {
    const current = status({ stage: "error", error: "Нет соединения" });
    expect(
      reconcileModelStatus(current, status({ stage: "consent_required" })).stage,
    ).toBe("error");
  });

  it("never moves the download progress backwards", () => {
    const current = status({ stage: "downloading", progress: 42, downloaded_bytes: 200 });
    const result = reconcileModelStatus(
      current,
      status({ stage: "downloading", progress: 0, downloaded_bytes: 0 }),
    );

    expect(result.progress).toBe(42);
    expect(result.downloaded_bytes).toBe(200);
  });

  it("accepts later stages", () => {
    expect(
      reconcileModelStatus(
        status({ stage: "downloading", progress: 99 }),
        status({ stage: "ready", progress: 100 }),
      ).stage,
    ).toBe("ready");
    expect(
      reconcileModelStatus(
        status({ stage: "loading" }),
        status({ stage: "error", error: "boom" }),
      ).stage,
    ).toBe("error");
  });
  it("keeps an error that is newer than a stale downloading reply", () => {
    const failed = status({ stage: "error", error: "Нет соединения", revision: 4 });
    const staleReply = status({ stage: "downloading", progress: 0, revision: 3 });

    expect(reconcileModelStatus(failed, staleReply).stage).toBe("error");
  });

  it("accepts a newer retry revision even when progress restarts", () => {
    const current = status({ stage: "downloading", progress: 99, revision: 5 });
    const retry = status({ stage: "downloading", progress: 0, revision: 6 });

    expect(reconcileModelStatus(current, retry).progress).toBe(0);
  });

});
