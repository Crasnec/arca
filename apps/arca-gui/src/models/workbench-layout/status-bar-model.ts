import type { WorkbenchLayoutProps } from "../../components/workbench";
import type { StatusBarModelInput } from "./types";

export function buildStatusBarModel({
  archiveState: { manifest },
  feedback: { status },
  operation: { activeOperation },
  archive: { table },
  directEdit: { pending },
  ui: { selectedCount },
  capabilities: { redoAvailable }
}: StatusBarModelInput): WorkbenchLayoutProps["statusBar"] {
  return {
    operation: activeOperation,
    summary: {
      selectedCount,
      entryFilterActive: table.entryFilterActive,
      visibleEntryCount: table.visibleEntryCount,
      filterableEntryCount: table.filterableEntryCount,
      pendingChangeCount: pending.pendingChangeCount,
      hasPendingChanges: pending.hasPendingChanges,
      redoAvailable,
      entryCount: manifest?.entryCount ?? null
    },
    archive: {
      manifest,
      hasPendingChanges: pending.hasPendingChanges
    },
    message: status
  };
}
