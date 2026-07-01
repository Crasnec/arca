import type React from "react";
import {
  buildWorkbenchActionGroups,
  type WorkbenchActionGroupInput
} from "../../models/workbench-actions";
import type {
  ArchiveManifest,
  CloseBlockedPromptState,
  DirectEditReplacePromptState,
  OverwritePromptState,
  PasswordAction,
  UnsavedPromptState
} from "../../shared/types";
import {
  buildWorkbenchPromptState,
  useWorkbenchBindings
} from "./workbench-bindings";
import {
  type WorkbenchDialogsInput,
  useWorkbenchDialogs
} from "./workbench-dialogs";

type WorkbenchInteractionsInput = {
  dialogs: WorkbenchDialogsInput;
  actions: Omit<WorkbenchActionGroupInput, "dialogs">;
  binding: {
    manifest: ArchiveManifest | null;
    loading: boolean;
    selectedCount: number;
    hasPendingChanges: boolean;
    pendingUndoCount: number;
    pendingRedoCount: number;
    createOpen: boolean;
    directEditReplacePrompt: DirectEditReplacePromptState | null;
    overwritePrompt: OverwritePromptState | null;
    unsavedPrompt: UnsavedPromptState | null;
    closeBlockedPrompt: CloseBlockedPromptState | null;
    passwordAction: PasswordAction | null;
    setStatus: React.Dispatch<React.SetStateAction<string>>;
  };
};

export function useWorkbenchInteractions({
  dialogs: dialogInput,
  actions,
  binding
}: WorkbenchInteractionsInput) {
  const dialogs = useWorkbenchDialogs(dialogInput);
  const actionGroups = buildWorkbenchActionGroups({
    ...actions,
    dialogs
  });
  const capabilities = useWorkbenchBindings({
    state: {
      manifest: binding.manifest,
      loading: binding.loading,
      selectedCount: binding.selectedCount,
      hasPendingChanges: binding.hasPendingChanges,
      pendingUndoCount: binding.pendingUndoCount,
      pendingRedoCount: binding.pendingRedoCount
    },
    prompts: buildWorkbenchPromptState({
      createOpen: binding.createOpen,
      directEditReplacePrompt: binding.directEditReplacePrompt,
      overwritePrompt: binding.overwritePrompt,
      unsavedPrompt: binding.unsavedPrompt,
      closeBlockedPrompt: binding.closeBlockedPrompt,
      passwordAction: binding.passwordAction,
      dialogs
    }),
    actions: actionGroups,
    setStatus: binding.setStatus
  });

  return {
    dialogs,
    actions: actionGroups,
    capabilities
  };
}

