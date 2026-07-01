import type { WorkbenchLayoutProps } from "../../components/workbench";
import type { CommandBarModelInput } from "./types";

export function buildCommandBarModel({
  feedback: { loading },
  operation: { activeOperation },
  directEdit: { pending },
  ui: { selectedCount, canAddDirectEdit },
  capabilities,
  actions
}: CommandBarModelInput): WorkbenchLayoutProps["commandBar"] {
  const { archive, browser, directEdit, dialogs, operation, pendingChanges } = actions;
  return {
    state: {
      hasArchive: capabilities.hasArchive,
      loading,
      selectedCount,
      canAddDirectEdit,
      canDeleteSelected: capabilities.canDeleteSelected,
      hasPendingChanges: pending.hasPendingChanges,
      canSaveDirectEdit: capabilities.canSaveDirectEdit,
      canUndoPendingChanges: capabilities.canUndoPendingChanges,
      canRedoPendingChanges: capabilities.canRedoPendingChanges,
      activeOperation
    },
    actions: {
      onAddFiles: directEdit.chooseDirectEditFiles,
      onExtract: () =>
        selectedCount > 0 ? archive.extractSelectedEntries() : archive.extractArchive(),
      onTest: () =>
        selectedCount > 0 ? archive.testSelectedEntries() : archive.testArchive(),
      onCopy: browser.copySelectedPaths,
      onDelete: directEdit.markSelectedForDelete,
      onInfo: dialogs.openArchiveInfo,
      onSave: pendingChanges.savePendingChanges,
      onUndo: pendingChanges.undoPendingChangeSet,
      onRedo: pendingChanges.redoPendingChangeSet,
      onCancelOperation: operation.cancelActiveOperation
    }
  };
}
