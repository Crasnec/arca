import { createCreateArchiveInputActions } from "./create-archive-input-actions";
import { createCreateArchiveModalActions } from "./create-archive-modal-actions";
import { createCreateArchiveSubmitAction } from "./create-archive-submit-action";
import type {
  CreateArchiveFeedback,
  CreateArchivePendingChanges,
  CreateArchivePrompts,
  CreateArchiveState,
  CreateArchiveTarget,
  RunCreateArchiveCommand
} from "./create-archive-action-types";

type CreateArchiveActionsInput = {
  state: CreateArchiveState;
  archive: CreateArchiveTarget;
  pendingChanges: CreateArchivePendingChanges;
  feedback: CreateArchiveFeedback;
  prompts: CreateArchivePrompts;
  runCreateArchiveCommand: RunCreateArchiveCommand;
};

export function createCreateArchiveActions({
  state,
  archive,
  pendingChanges,
  feedback,
  prompts,
  runCreateArchiveCommand
}: CreateArchiveActionsInput) {
  const modalActions = createCreateArchiveModalActions({
    state,
    pendingChanges,
    feedback,
    prompts
  });
  const inputActions = createCreateArchiveInputActions({
    state,
    feedback
  });
  const submitAction = createCreateArchiveSubmitAction({
    state,
    archive,
    pendingChanges,
    feedback,
    prompts,
    runCreateArchiveCommand
  });

  return {
    ...modalActions,
    ...inputActions,
    ...submitAction
  };
}
