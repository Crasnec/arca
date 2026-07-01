import type {
  DirectEditPendingActionsContext,
  DirectEditPendingFeedback
} from "./direct-edit-pending-action-types";

type DirectEditPendingHistoryActionsInput = Pick<
  DirectEditPendingActionsContext,
  "pendingChanges" | "setDirectEditReplacePrompt" | "resetSelection"
> &
  DirectEditPendingFeedback;

export function createDirectEditPendingHistoryActions({
  pendingChanges,
  setDirectEditReplacePrompt,
  resetSelection,
  setError,
  setStatus
}: DirectEditPendingHistoryActionsInput) {
  function resetAllPendingChanges() {
    pendingChanges.resetAllPendingChanges();
    setDirectEditReplacePrompt(null);
  }

  function undoPendingChangeSet() {
    if (!pendingChanges.undoPendingChangeSet()) {
      return;
    }
    setDirectEditReplacePrompt(null);
    resetSelection();
    setError(null);
    setStatus("Pending changes undone");
  }

  function redoPendingChangeSet() {
    if (!pendingChanges.redoPendingChangeSet()) {
      return;
    }
    setDirectEditReplacePrompt(null);
    resetSelection();
    setError(null);
    setStatus("Pending changes restored");
  }

  return {
    resetAllPendingChanges,
    undoPendingChangeSet,
    redoPendingChangeSet
  };
}
