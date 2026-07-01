import type { createDirectEditAddActions } from "./direct-edit-add-actions";
import type { createDirectEditPendingActions } from "./direct-edit-pending-actions";
import type { useDirectEditReplacePrompt } from "./direct-edit-replace-prompt";
import type { createDirectEditFeedbackActions } from "./direct-edit-feedback-actions";
import type { usePendingChanges } from "./pending-changes";

type PendingChanges = ReturnType<typeof usePendingChanges>;
type ReplacementPrompt = ReturnType<typeof useDirectEditReplacePrompt>;
type AddActions = ReturnType<typeof createDirectEditAddActions>;
type PendingActions = ReturnType<typeof createDirectEditPendingActions>;
type FeedbackActions = ReturnType<typeof createDirectEditFeedbackActions>;

type DirectEditWorkflowModelInput = {
  pendingChanges: PendingChanges;
  replacement: ReplacementPrompt;
  addActions: AddActions;
  pendingActions: PendingActions;
  feedbackActions: Pick<FeedbackActions, "appendPendingAddPlan">;
};

export function buildDirectEditWorkflowModel({
  pendingChanges,
  replacement,
  addActions,
  pendingActions,
  feedbackActions
}: DirectEditWorkflowModelInput) {
  const {
    pendingDeletePaths,
    pendingAddInputs,
    pendingAddEntries,
    pendingReplaceEntries,
    pendingUndoStack,
    pendingRedoStack,
    pendingChangeCount,
    hasPendingChanges
  } = pendingChanges;
  const {
    directEditReplacePrompt,
    closeDirectEditReplacePrompt,
    skipReplacementAddition,
    confirmReplacementAddition,
    skipReplacementAdditions,
    confirmReplacementAdditions
  } = replacement;

  return {
    pending: {
      pendingDeletePaths,
      pendingAddInputs,
      pendingAddEntries,
      pendingReplaceEntries,
      pendingUndoStack,
      pendingRedoStack,
      pendingChangeCount,
      hasPendingChanges
    },
    replacement: {
      directEditReplacePrompt,
      closeDirectEditReplacePrompt,
      skipReplacementAddition,
      confirmReplacementAddition,
      skipReplacementAdditions,
      confirmReplacementAdditions
    },
    actions: {
      ...addActions,
      appendPendingAddPlan: feedbackActions.appendPendingAddPlan,
      ...pendingActions
    }
  };
}
