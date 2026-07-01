import type { useArchiveSessionWorkflow } from "../archive/archive-session-workflow";
import type { useArchiveViewState } from "../archive/archive-view-state";
import type { useCreateArchiveWorkflow } from "../create-archive/create-archive-workflow";
import type { useDirectEditWorkflow } from "../direct-edit/direct-edit-workflow";
import type { DropState } from "../../shared/types";
import type { useEntryBrowserState } from "./entry-browser-state";
import type {
  WorkbenchArchiveOpenAccess,
  WorkbenchPendingChangeAccess
} from "./workbench-archive-access";

type EntryBrowserState = ReturnType<typeof useEntryBrowserState>;
type ArchiveSessionWorkflow = ReturnType<typeof useArchiveSessionWorkflow>;
type ArchiveViewState = ReturnType<typeof useArchiveViewState>;
type CreateArchiveWorkflow = ReturnType<typeof useCreateArchiveWorkflow>;
type DirectEditWorkflow = ReturnType<typeof useDirectEditWorkflow>;

type WorkbenchArchiveWorkflowModelInput = {
  entryBrowser: EntryBrowserState;
  archiveSession: ArchiveSessionWorkflow;
  archiveView: ArchiveViewState;
  archiveOpenAccess: WorkbenchArchiveOpenAccess;
  createWorkflow: CreateArchiveWorkflow;
  directEditWorkflow: DirectEditWorkflow;
  pendingChangeAccess: WorkbenchPendingChangeAccess;
  ui: {
    dropState: DropState;
    canAddDirectEdit: boolean;
  };
};

export function buildWorkbenchArchiveWorkflowModel({
  entryBrowser,
  archiveSession,
  archiveView,
  archiveOpenAccess,
  createWorkflow,
  directEditWorkflow,
  pendingChangeAccess,
  ui: { dropState, canAddDirectEdit }
}: WorkbenchArchiveWorkflowModelInput) {
  const {
    entryFilter,
    entrySort,
    selectedPaths,
    entryFilterRef,
    updateEntryFilter,
    clearEntryFilter,
    changeEntrySort,
    selectEntry,
    openEntryContextMenu,
    copySelectedPaths,
    focusEntryFilter
  } = entryBrowser;
  const {
    open: archiveOpen,
    startup: archiveStartup,
    payload: archivePayload
  } = archiveSession;
  const { state: createState, actions: createActions } = createWorkflow;
  const {
    pending: directEditPending,
    replacement: directEditReplacement,
    actions: directEditActions
  } = directEditWorkflow;

  return {
    browser: {
      entryFilter,
      entrySort,
      entryFilterRef,
      updateEntryFilter,
      clearEntryFilter,
      changeEntrySort,
      selectEntry,
      openEntryContextMenu
    },
    browserActions: {
      copySelectedPaths,
      focusEntryFilter
    },
    archive: {
      open: archiveOpen,
      startup: archiveStartup,
      payload: archivePayload,
      openAccess: archiveOpenAccess,
      table: archiveView.table,
      selection: archiveView.selection
    },
    create: {
      state: createState,
      actions: createActions
    },
    directEdit: {
      pending: directEditPending,
      pendingChangeAccess,
      replacement: directEditReplacement,
      actions: directEditActions
    },
    ui: {
      dropState,
      selectedCount: selectedPaths.length,
      canAddDirectEdit
    }
  };
}
