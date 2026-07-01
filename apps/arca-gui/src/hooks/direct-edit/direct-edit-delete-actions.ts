import type {
  DirectEditPendingActionsContext,
  DirectEditPendingFeedback
} from "./direct-edit-pending-action-types";

type DirectEditDeleteActionsInput = Pick<
  DirectEditPendingActionsContext,
  "manifest" | "selectedPaths" | "pendingChanges" | "resetSelection" | "reportDirectEditUnavailable"
> &
  DirectEditPendingFeedback;

export function createDirectEditDeleteActions({
  manifest,
  selectedPaths,
  pendingChanges,
  resetSelection,
  reportDirectEditUnavailable,
  setError,
  setStatus
}: DirectEditDeleteActionsInput) {
  function markSelectedForDelete() {
    if (!manifest?.directEdit.allowed) {
      reportDirectEditUnavailable("Delete unavailable");
      return;
    }
    if (selectedPaths.length === 0) {
      setError("Select archive entries before deleting");
      setStatus("Delete failed");
      return;
    }
    const result = pendingChanges.markPathsForDelete(selectedPaths);
    if (result === "unchanged") {
      setStatus("Entries already pending delete");
      return;
    }
    resetSelection();
    setError(null);
    setStatus(
      `${selectedPaths.length} entr${selectedPaths.length === 1 ? "y" : "ies"} pending delete`
    );
  }

  return {
    markSelectedForDelete
  };
}
