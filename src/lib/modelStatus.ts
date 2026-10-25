import type { ModelStatus } from "../types";

const ADVANCED_STAGES = new Set<ModelStatus["stage"]>([
  "downloading",
  "loading",
  "ready",
  "error",
]);

/**
 * Model status arrives from two independent channels: the invoke reply and the
 * `model-status` event. Either can arrive first, so the manager stamps every
 * status with a monotonic revision and the UI keeps the newest one. Without
 * this, a fast network error could be overwritten by the older reply and the
 * permission button would look dead forever.
 */
export function reconcileModelStatus(
  current: ModelStatus | null,
  incoming: ModelStatus,
): ModelStatus {
  if (!current) return incoming;

  const currentRevision = current.revision ?? 0;
  const incomingRevision = incoming.revision ?? 0;

  if (incomingRevision < currentRevision) return current;
  if (incomingRevision > currentRevision) return incoming;

  // Same revision: both values describe the same backend state.
  if (incoming.stage === "consent_required" && ADVANCED_STAGES.has(current.stage)) {
    return current;
  }

  if (current.stage === "downloading" && incoming.stage === "downloading") {
    return {
      ...incoming,
      progress: Math.max(current.progress, incoming.progress),
      downloaded_bytes: Math.max(
        current.downloaded_bytes ?? 0,
        incoming.downloaded_bytes ?? 0,
      ),
      total_bytes: incoming.total_bytes ?? current.total_bytes,
    };
  }

  return incoming;
}
