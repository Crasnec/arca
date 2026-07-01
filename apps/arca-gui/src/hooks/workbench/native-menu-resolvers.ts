import type { WorkbenchActionGroups } from "../../models/workbench-actions";
import { OPEN_SETTINGS_EVENT } from "../../shared/constants";

export type NativeMenuCapabilities = {
  canSaveDirectEdit: boolean;
  canUndoPendingChanges: boolean;
  canRedoPendingChanges: boolean;
};

type NativeMenuActionInput = {
  action: string;
  capabilities: NativeMenuCapabilities;
  selectedCount: number;
  actions: WorkbenchActionGroups;
};

export function runNativeMenuAction({
  action,
  capabilities,
  selectedCount,
  actions
}: NativeMenuActionInput) {
  const { archive, browser, create, directEdit, dialogs, pendingChanges } = actions;
  if (action === "arca-menu-open") {
    void archive.chooseArchive();
  } else if (action === "arca-menu-new") {
    create.openCreateModal();
  } else if (action === "arca-menu-add-files") {
    void directEdit.chooseDirectEditFiles();
  } else if (action === "arca-menu-extract") {
    selectedCount > 0 ? void archive.extractSelectedEntries() : void archive.extractArchive();
  } else if (action === "arca-menu-test") {
    selectedCount > 0 ? void archive.testSelectedEntries() : void archive.testArchive();
  } else if (action === "arca-menu-copy") {
    void browser.copySelectedPaths();
  } else if (action === "arca-menu-delete") {
    directEdit.markSelectedForDelete();
  } else if (action === "arca-menu-info") {
    dialogs.openArchiveInfo();
  } else if (action === "arca-menu-find") {
    browser.focusEntryFilter();
  } else if (action === "arca-menu-settings") {
    window.dispatchEvent(new Event(OPEN_SETTINGS_EVENT));
  } else if (action === "arca-menu-save" && capabilities.canSaveDirectEdit) {
    void pendingChanges.savePendingChanges();
  } else if (action === "arca-menu-undo" && capabilities.canUndoPendingChanges) {
    pendingChanges.undoPendingChangeSet();
  } else if (action === "arca-menu-redo" && capabilities.canRedoPendingChanges) {
    pendingChanges.redoPendingChangeSet();
  } else if (action === "arca-context-extract-selection") {
    void archive.extractSelectedEntries();
  } else if (action === "arca-context-extract-here") {
    void archive.extractSelectedEntriesHere();
  } else if (action === "arca-context-test-selection") {
    void archive.testSelectedEntries();
  } else if (action === "arca-context-add-files") {
    void directEdit.chooseDirectEditFiles();
  } else if (action === "arca-context-add-folder") {
    void directEdit.chooseDirectEditFolder();
  } else if (action === "arca-context-copy-path") {
    void browser.copySelectedPaths();
  } else if (action === "arca-context-delete") {
    directEdit.markSelectedForDelete();
  } else if (action === "arca-context-properties") {
    dialogs.openSelectedEntryInfo();
  }
}
