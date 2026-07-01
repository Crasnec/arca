import { pendingChangesMessage } from "../../shared/pending-change-utils";
import type {
  CreateArchiveFeedback,
  CreateArchivePendingChanges,
  CreateArchivePrompts,
  CreateArchiveState
} from "./create-archive-action-types";

type CreateArchiveModalActionsInput = {
  state: Pick<CreateArchiveState, "showCreateModal" | "closeCreateModal">;
  pendingChanges: Pick<
    CreateArchivePendingChanges,
    "hasPendingChanges" | "pendingChangeCount"
  >;
  feedback: CreateArchiveFeedback;
  prompts: Pick<CreateArchivePrompts, "setUnsavedPrompt">;
};

export function createCreateArchiveModalActions({
  state,
  pendingChanges: { hasPendingChanges, pendingChangeCount },
  feedback: { setError, setStatus },
  prompts: { setUnsavedPrompt }
}: CreateArchiveModalActionsInput) {
  function openCreateModal() {
    if (hasPendingChanges) {
      setUnsavedPrompt({
        action: { kind: "newArchive" },
        message: pendingChangesMessage(pendingChangeCount)
      });
      setStatus("Unsaved changes");
      return;
    }
    showCreateModal();
  }

  function showCreateModal() {
    setError(null);
    state.showCreateModal();
    setStatus("New archive");
  }

  function closeCreateModal() {
    state.closeCreateModal();
    setStatus("Ready");
  }

  return {
    openCreateModal,
    showCreateModal,
    closeCreateModal
  };
}
