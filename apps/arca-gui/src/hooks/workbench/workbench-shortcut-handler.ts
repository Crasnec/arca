import type { WorkbenchActionGroups } from "../../models/workbench-actions";
import { isTextEditingShortcutTarget } from "../../shared/dom-utils";

export type ShortcutPromptState = {
  createOpen: boolean;
  directEditReplacePromptOpen: boolean;
  overwritePromptOpen: boolean;
  unsavedPromptOpen: boolean;
  closeBlockedPromptOpen: boolean;
  infoOpen: boolean;
  entryInfoOpen: boolean;
  passwordPromptOpen: boolean;
};

export type ShortcutCapabilities = {
  canSaveDirectEdit: boolean;
  canUndoPendingChanges: boolean;
  canRedoPendingChanges: boolean;
  canDeleteSelected: boolean;
};

type WorkbenchShortcutHandlerInput = {
  prompts: ShortcutPromptState;
  loading: boolean;
  capabilities: ShortcutCapabilities;
  actions: WorkbenchActionGroups;
};

function hasModalOpen(prompts: ShortcutPromptState) {
  return Boolean(
    prompts.createOpen ||
      prompts.passwordPromptOpen ||
      prompts.overwritePromptOpen ||
      prompts.directEditReplacePromptOpen ||
      prompts.unsavedPromptOpen ||
      prompts.closeBlockedPromptOpen ||
      prompts.infoOpen ||
      prompts.entryInfoOpen
  );
}

function handleEscapeShortcut({
  prompts,
  loading,
  actions: { create, dialogs }
}: Pick<WorkbenchShortcutHandlerInput, "prompts" | "loading" | "actions">) {
  if (prompts.directEditReplacePromptOpen) {
    dialogs.closeDirectEditReplacePrompt();
  } else if (prompts.overwritePromptOpen) {
    dialogs.closeOverwritePrompt();
  } else if (prompts.unsavedPromptOpen) {
    dialogs.closeUnsavedPrompt();
  } else if (prompts.closeBlockedPromptOpen) {
    dialogs.closeCloseBlockedPrompt();
  } else if (prompts.infoOpen) {
    dialogs.closeArchiveInfo();
  } else if (prompts.entryInfoOpen) {
    dialogs.closeSelectedEntryInfo();
  } else if (prompts.passwordPromptOpen) {
    dialogs.closePasswordPrompt();
  } else if (prompts.createOpen && !loading) {
    create.closeCreateModal();
  }
}

function handleCommandShortcut({
  event,
  key,
  modKey,
  capabilities,
  actions: { archive, browser, create, directEdit, pendingChanges }
}: Pick<WorkbenchShortcutHandlerInput, "capabilities" | "actions"> & {
  event: KeyboardEvent;
  key: string;
  modKey: boolean;
}) {
  if (modKey && key === "f") {
    event.preventDefault();
    browser.focusEntryFilter();
    return;
  }

  if (!modKey && isTextEditingShortcutTarget(event.target)) {
    return;
  }

  if (modKey && key === "o") {
    event.preventDefault();
    void archive.chooseArchive();
  } else if (modKey && key === "n") {
    event.preventDefault();
    create.openCreateModal();
  } else if (modKey && key === "s") {
    event.preventDefault();
    if (capabilities.canSaveDirectEdit) {
      void pendingChanges.savePendingChanges();
    }
  } else if (modKey && key === "z" && event.shiftKey) {
    event.preventDefault();
    if (capabilities.canRedoPendingChanges) {
      pendingChanges.redoPendingChangeSet();
    }
  } else if (modKey && key === "z") {
    event.preventDefault();
    if (capabilities.canUndoPendingChanges) {
      pendingChanges.undoPendingChangeSet();
    }
  } else if (modKey && key === "y") {
    event.preventDefault();
    if (capabilities.canRedoPendingChanges) {
      pendingChanges.redoPendingChangeSet();
    }
  } else if (!modKey && event.key === "Delete" && capabilities.canDeleteSelected) {
    event.preventDefault();
    directEdit.markSelectedForDelete();
  }
}

export function createWorkbenchShortcutHandler({
  prompts,
  loading,
  capabilities,
  actions
}: WorkbenchShortcutHandlerInput) {
  return function handleWorkbenchShortcut(event: KeyboardEvent) {
    const key = event.key.toLowerCase();
    const modKey = event.ctrlKey || event.metaKey;

    if (event.key === "Escape") {
      handleEscapeShortcut({ prompts, loading, actions });
      return;
    }

    if (hasModalOpen(prompts) || loading) {
      return;
    }

    handleCommandShortcut({
      event,
      key,
      modKey,
      capabilities,
      actions
    });
  };
}
