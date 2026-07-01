import type { WorkbenchLayoutProps } from "../../components/workbench";
import type { EntryWorkspaceModelInput } from "./types";

export function buildEntryWorkspaceModel({
  archiveState: { manifest },
  browser: {
    entrySort,
    changeEntrySort,
    selectEntry,
    openEntryContextMenu
  },
  archive: { table },
  directEdit: { pending },
  ui: { dropState }
}: EntryWorkspaceModelInput): WorkbenchLayoutProps["entryWorkspace"] {
  return {
    view: {
      dropState,
      manifest,
      treeRows: table.treeRows,
      table: {
        sort: entrySort,
        visibleEntryCount: table.visibleEntryCount,
        filterActive: table.entryFilterActive,
        entries: table.visibleEntries,
        pendingAddEntries: table.visiblePendingAddEntries,
        selectedPathSet: table.selectedPathSet,
        pendingReplaceEntries: pending.pendingReplaceEntries
      }
    },
    actions: {
      onSort: changeEntrySort,
      onSelectEntry: (path, event) => selectEntry(path, event, table.visibleEntries),
      onOpenEntryContextMenu: openEntryContextMenu
    }
  };
}
