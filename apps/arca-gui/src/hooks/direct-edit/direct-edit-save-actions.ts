import type {
  DirectEditPendingActionsContext,
  DirectEditPendingFeedback,
  DirectEditPendingState
} from "./direct-edit-pending-action-types";

type DirectEditSaveActionsInput = Pick<
  DirectEditPendingActionsContext,
  "manifest" | "setManifest" | "resetSelection" | "reportDirectEditUnavailable" | "runSaveDirectEdit"
> & {
  pending: DirectEditPendingState;
  resetAllPendingChanges: () => void;
} & DirectEditPendingFeedback;

export function createDirectEditSaveActions({
  manifest,
  pending: {
    pendingDeletePaths,
    pendingAddInputs,
    pendingAddEntries,
    pendingReplaceEntries,
    hasPendingChanges
  },
  setManifest,
  resetSelection,
  reportDirectEditUnavailable,
  runSaveDirectEdit,
  resetAllPendingChanges,
  setError,
  setStatus
}: DirectEditSaveActionsInput) {
  async function savePendingChanges() {
    if (!manifest || !hasPendingChanges) {
      setError("No pending changes to save");
      setStatus("Save failed");
      return;
    }
    if (!manifest.directEdit.allowed) {
      reportDirectEditUnavailable("Save failed");
      return;
    }

    const value = await runSaveDirectEdit({
      archivePath: manifest.archivePath,
      expectedDigestSha256: manifest.digestSha256,
      deleteEntries: pendingDeletePaths,
      addInputs: pendingAddInputs,
      addEntries: pendingAddEntries.map((entry) => entry.archivePath),
      replaceEntries: pendingReplaceEntries
    });
    if (!value) {
      return;
    }
    setManifest(value);
    resetAllPendingChanges();
    resetSelection();
    setStatus(`Saved ${value.archiveName}`);
  }

  return {
    savePendingChanges
  };
}
