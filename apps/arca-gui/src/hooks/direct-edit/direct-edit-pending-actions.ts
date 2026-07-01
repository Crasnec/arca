import { createDirectEditDeleteActions } from "./direct-edit-delete-actions";
import { createDirectEditPendingHistoryActions } from "./direct-edit-pending-history-actions";
import { createDirectEditSaveActions } from "./direct-edit-save-actions";
import type { DirectEditPendingActionsContext } from "./direct-edit-pending-action-types";

export function createDirectEditPendingActions(input: DirectEditPendingActionsContext) {
  const historyActions = createDirectEditPendingHistoryActions(input);
  return {
    ...createDirectEditDeleteActions(input),
    ...historyActions,
    ...createDirectEditSaveActions({
      ...input,
      resetAllPendingChanges: historyActions.resetAllPendingChanges
    })
  };
}
