import type { ArchiveManifest, DirectEditPlannedEntry } from "../../shared/types";
import type { FeedbackPort } from "../workflow-ports";
import type { usePendingChanges } from "./pending-changes";

type DirectEditFeedbackActionsInput = {
  manifest: ArchiveManifest | null;
  pendingChanges: Pick<ReturnType<typeof usePendingChanges>, "appendPendingAddPlan">;
  feedback: Pick<FeedbackPort, "setError" | "setStatus">;
};

export function createDirectEditFeedbackActions({
  manifest,
  pendingChanges,
  feedback: { setError, setStatus }
}: DirectEditFeedbackActionsInput) {
  function appendPendingAddPlan(
    inputs: string[],
    additions: DirectEditPlannedEntry[],
    replacements: DirectEditPlannedEntry[]
  ) {
    const entryCount = pendingChanges.appendPendingAddPlan(inputs, additions, replacements);
    if (entryCount === 0) {
      setStatus("Add skipped");
      return;
    }
    setError(null);
    setStatus(`${entryCount} entr${entryCount === 1 ? "y" : "ies"} pending add`);
  }

  function reportDirectEditUnavailable(unavailableStatus: string) {
    setError(manifest?.directEdit.reason ?? "Direct Editing is not available for this archive");
    setStatus(unavailableStatus);
  }

  return {
    appendPendingAddPlan,
    reportDirectEditUnavailable
  };
}
