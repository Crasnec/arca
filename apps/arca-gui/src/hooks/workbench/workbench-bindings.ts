import type React from "react";
import { useNativeMenuActions } from "./native-menu-actions";
import { useWorkbenchShortcuts } from "./workbench-shortcuts";
import type { WorkbenchActionGroups } from "../../models/workbench-actions";
import type {
  ArchiveManifest,
  CloseBlockedPromptState,
  DirectEditReplacePromptState,
  OverwritePromptState,
  PasswordAction,
  UnsavedPromptState
} from "../../shared/types";

type WorkbenchBindingState = {
  manifest: ArchiveManifest | null;
  loading: boolean;
  selectedCount: number;
  hasPendingChanges: boolean;
  pendingUndoCount: number;
  pendingRedoCount: number;
};

export type WorkbenchPromptState = {
  createOpen: boolean;
  directEditReplacePromptOpen: boolean;
  overwritePromptOpen: boolean;
  unsavedPromptOpen: boolean;
  closeBlockedPromptOpen: boolean;
  infoOpen: boolean;
  entryInfoOpen: boolean;
  passwordPromptOpen: boolean;
};

type WorkbenchPromptStateInput = {
  createOpen: boolean;
  directEditReplacePrompt: DirectEditReplacePromptState | null;
  overwritePrompt: OverwritePromptState | null;
  unsavedPrompt: UnsavedPromptState | null;
  closeBlockedPrompt: CloseBlockedPromptState | null;
  passwordAction: PasswordAction | null;
  dialogs: {
    archiveInfo: { open: boolean };
    entryInfo: { open: boolean };
  };
};

type WorkbenchBindingsInput = {
  state: WorkbenchBindingState;
  prompts: WorkbenchPromptState;
  actions: WorkbenchActionGroups;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
};

export type WorkbenchCapabilities = {
  hasArchive: boolean;
  redoAvailable: boolean;
  canSaveDirectEdit: boolean;
  canDeleteSelected: boolean;
  canUndoPendingChanges: boolean;
  canRedoPendingChanges: boolean;
};

function buildWorkbenchCapabilities({
  manifest,
  loading,
  selectedCount,
  hasPendingChanges,
  pendingUndoCount,
  pendingRedoCount
}: WorkbenchBindingState): WorkbenchCapabilities {
  return {
    hasArchive: Boolean(manifest),
    redoAvailable: pendingRedoCount > 0,
    canSaveDirectEdit: Boolean(manifest?.directEdit.allowed && hasPendingChanges && !loading),
    canDeleteSelected: Boolean(!loading && selectedCount > 0 && manifest?.directEdit.allowed),
    canUndoPendingChanges: pendingUndoCount > 0 && !loading,
    canRedoPendingChanges: pendingRedoCount > 0 && !loading
  };
}

export function buildWorkbenchPromptState({
  createOpen,
  directEditReplacePrompt,
  overwritePrompt,
  unsavedPrompt,
  closeBlockedPrompt,
  passwordAction,
  dialogs
}: WorkbenchPromptStateInput): WorkbenchPromptState {
  return {
    createOpen,
    directEditReplacePromptOpen: Boolean(directEditReplacePrompt),
    overwritePromptOpen: Boolean(overwritePrompt),
    unsavedPromptOpen: Boolean(unsavedPrompt),
    closeBlockedPromptOpen: Boolean(closeBlockedPrompt),
    infoOpen: dialogs.archiveInfo.open,
    entryInfoOpen: dialogs.entryInfo.open,
    passwordPromptOpen: Boolean(passwordAction)
  };
}

export function useWorkbenchBindings({
  state,
  prompts,
  actions,
  setStatus
}: WorkbenchBindingsInput): WorkbenchCapabilities {
  const capabilities = buildWorkbenchCapabilities(state);

  useNativeMenuActions({
    capabilities,
    selectedCount: state.selectedCount,
    setStatus,
    actions
  });

  useWorkbenchShortcuts({
    prompts,
    loading: state.loading,
    capabilities,
    actions
  });

  return capabilities;
}
