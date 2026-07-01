import type { useOperationTracker } from "./operation-tracker";
import type { useWorkbenchArchiveWorkflows } from "./workbench-archive-workflows";
import type { useWorkbenchInteractions } from "./workbench-interactions";
import type { buildWorkbenchPorts } from "./workbench-ports";
import type { useWorkbenchState } from "./workbench-state";

type WorkbenchState = ReturnType<typeof useWorkbenchState>;
type WorkbenchPorts = ReturnType<typeof buildWorkbenchPorts>;
type OperationTracker = ReturnType<typeof useOperationTracker>;
type WorkbenchArchiveWorkflows = ReturnType<typeof useWorkbenchArchiveWorkflows>;
type WorkbenchInteractions = ReturnType<typeof useWorkbenchInteractions>;

type WorkbenchInteractionInputSource = {
  archiveState: WorkbenchState["archive"];
  feedback: WorkbenchState["feedback"];
  prompts: WorkbenchState["prompts"];
  ports: Pick<WorkbenchPorts, "infoFeedback" | "statusFeedback" | "promptDialogState">;
  workflows: WorkbenchArchiveWorkflows;
  operation: Pick<OperationTracker, "cancelActiveOperation">;
};

type WorkbenchWorkflowModelInput = {
  archiveState: WorkbenchState["archive"];
  feedback: WorkbenchState["feedback"];
  prompts: WorkbenchState["prompts"];
  refs: WorkbenchState["refs"];
  operation: Pick<OperationTracker, "activeOperation">;
  workflows: WorkbenchArchiveWorkflows;
  interactions: WorkbenchInteractions;
};

export function buildWorkbenchInteractionInput({
  archiveState,
  feedback,
  prompts,
  ports: { infoFeedback, statusFeedback, promptDialogState },
  workflows: { archive, browserActions, create, directEdit, ui },
  operation: { cancelActiveOperation }
}: WorkbenchInteractionInputSource) {
  const { manifest } = archiveState;
  const { loading } = feedback;
  const {
    passwordAction,
    overwritePrompt,
    unsavedPrompt,
    closeBlockedPrompt
  } = prompts;
  const {
    open: archiveOpen,
    startup: archiveStartup,
    payload: archivePayload
  } = archive;
  const { state: createState, actions: createActions } = create;
  const {
    pending: directEditPending,
    pendingChangeAccess,
    replacement: directEditReplacement,
    actions: directEditActions
  } = directEdit;

  return {
    dialogs: {
      info: {
        manifest,
        selectedEntries: archive.selection.selectedEntries,
        ...infoFeedback
      },
      prompts: promptDialogState,
      archive: archive.openAccess,
      pendingChanges: pendingChangeAccess,
      startup: archiveStartup,
      create: createActions,
      archivePayload,
      feedback: statusFeedback
    },
    actions: {
      archiveOpen,
      archivePayload,
      browser: browserActions,
      create: createActions,
      directEdit: directEditActions,
      directEditReplacement,
      operation: {
        cancelActiveOperation
      }
    },
    binding: {
      manifest,
      loading,
      selectedCount: ui.selectedCount,
      hasPendingChanges: directEditPending.hasPendingChanges,
      pendingUndoCount: directEditPending.pendingUndoStack.length,
      pendingRedoCount: directEditPending.pendingRedoStack.length,
      createOpen: createState.createOpen,
      directEditReplacePrompt: directEditReplacement.directEditReplacePrompt,
      overwritePrompt,
      unsavedPrompt,
      closeBlockedPrompt,
      passwordAction,
      setStatus: feedback.setStatus
    }
  };
}

export function buildWorkbenchWorkflowModel({
  archiveState,
  feedback,
  prompts,
  refs,
  operation: { activeOperation },
  workflows: { browser, archive, create, directEdit, ui },
  interactions: { dialogs, actions, capabilities }
}: WorkbenchWorkflowModelInput) {
  return {
    archiveState,
    feedback,
    prompts,
    refs,
    operation: {
      activeOperation
    },
    browser,
    archive: {
      open: archive.open,
      payload: archive.payload,
      table: archive.table,
      selection: archive.selection
    },
    create: {
      state: create.state,
      actions: create.actions
    },
    directEdit: {
      pending: directEdit.pending,
      replacement: directEdit.replacement
    },
    ui: {
      dropState: ui.dropState,
      selectedCount: ui.selectedCount,
      canAddDirectEdit: ui.canAddDirectEdit
    },
    capabilities,
    actions,
    dialogs
  };
}
