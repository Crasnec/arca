import type { useArchiveSessionWorkflow } from "../archive/archive-session-workflow";
import type { useDirectEditWorkflow } from "../direct-edit/direct-edit-workflow";
import type { useEntryBrowserState } from "./entry-browser-state";
import type { useWorkbenchState } from "./workbench-state";

type EntryBrowserState = ReturnType<typeof useEntryBrowserState>;
type DirectEditWorkflow = ReturnType<typeof useDirectEditWorkflow>;
type ArchiveOpenWorkflow = ReturnType<typeof useArchiveSessionWorkflow>["open"];
type WorkbenchArchiveState = ReturnType<typeof useWorkbenchState>["archive"];

export function buildSelectionAccess({
  selectedPaths,
  resetSelection
}: Pick<EntryBrowserState, "selectedPaths" | "resetSelection">) {
  return {
    selectedPaths,
    resetSelection
  };
}

export function buildPendingChangeAccess({
  pending,
  actions
}: Pick<DirectEditWorkflow, "pending" | "actions">) {
  return {
    hasPendingChanges: pending.hasPendingChanges,
    pendingChangeCount: pending.pendingChangeCount,
    resetAllPendingChanges: actions.resetAllPendingChanges
  };
}

export function buildArchiveOpenAccess(
  archiveState: Pick<WorkbenchArchiveState, "setArchivePath">,
  archiveOpen: Pick<ArchiveOpenWorkflow, "openArchivePath">
) {
  return {
    setArchivePath: archiveState.setArchivePath,
    openArchivePath: archiveOpen.openArchivePath
  };
}

export type WorkbenchSelectionAccess = ReturnType<typeof buildSelectionAccess>;
export type WorkbenchPendingChangeAccess = ReturnType<typeof buildPendingChangeAccess>;
export type WorkbenchArchiveOpenAccess = ReturnType<typeof buildArchiveOpenAccess>;
