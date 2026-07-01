import { useArchiveSessionWorkflow } from "../archive/archive-session-workflow";
import { useArchiveViewState } from "../archive/archive-view-state";
import { useCreateArchiveWorkflow } from "../create-archive/create-archive-workflow";
import { useDirectEditWorkflow } from "../direct-edit/direct-edit-workflow";
import { useEntryBrowserState } from "./entry-browser-state";
import {
  buildArchiveOpenAccess,
  buildPendingChangeAccess,
  buildSelectionAccess
} from "./workbench-archive-access";
import { buildWorkbenchArchiveWorkflowModel } from "./workbench-archive-workflow-model";
import { useWorkbenchEvents } from "./workbench-events";
import type { buildWorkbenchPorts } from "./workbench-ports";
import type { useWorkbenchState } from "./workbench-state";

type WorkbenchState = ReturnType<typeof useWorkbenchState>;
type WorkbenchPorts = ReturnType<typeof buildWorkbenchPorts>;

type WorkbenchArchiveWorkflowsInput = {
  archiveState: WorkbenchState["archive"];
  feedback: WorkbenchState["feedback"];
  prompts: WorkbenchState["prompts"];
  ports: Pick<
    WorkbenchPorts,
    "operation" | "feedbackActions" | "promptActions" | "archiveAccess"
  >;
};

export function useWorkbenchArchiveWorkflows({
  archiveState,
  feedback,
  prompts,
  ports: { operation, feedbackActions, promptActions, archiveAccess }
}: WorkbenchArchiveWorkflowsInput) {
  const { manifest } = archiveState;
  const { loading } = feedback;
  const canAddDirectEdit = Boolean(manifest?.directEdit.allowed && !loading);
  const entryBrowser = useEntryBrowserState({
    manifest,
    canAddDirectEdit,
    loading,
    setError: feedback.setError,
    setStatus: feedback.setStatus
  });
  const selectionAccess = buildSelectionAccess(entryBrowser);
  const directEditWorkflow = useDirectEditWorkflow({
    archive: archiveAccess,
    selection: selectionAccess,
    capability: {
      canAddDirectEdit
    },
    operation,
    feedback: feedbackActions
  });
  const pendingChangeAccess = buildPendingChangeAccess(directEditWorkflow);
  const archiveSession = useArchiveSessionWorkflow({
    archive: {
      path: archiveState.archivePath,
      destinationPath: archiveState.destinationPath,
      ...archiveAccess,
      setPath: archiveState.setArchivePath,
      setDestinationPath: archiveState.setDestinationPath
    },
    selection: {
      ...selectionAccess,
      resetEntryBrowserState: entryBrowser.resetEntryBrowserState
    },
    pendingChanges: pendingChangeAccess,
    operation,
    feedback: feedbackActions,
    prompts: promptActions
  });
  const archiveOpenAccess = buildArchiveOpenAccess(archiveState, archiveSession.open);
  const createWorkflow = useCreateArchiveWorkflow({
    state: {
      loading
    },
    archive: archiveOpenAccess,
    pendingChanges: pendingChangeAccess,
    operation,
    feedback: feedbackActions,
    prompts: promptActions
  });
  const dropState = useWorkbenchEvents({
    createOpen: createWorkflow.state.createOpen,
    directEditAllowed: Boolean(manifest?.directEdit.allowed),
    setStatus: feedback.setStatus,
    setCloseBlockedPrompt: prompts.setCloseBlockedPrompt,
    appendCreateInputs: createWorkflow.actions.appendCreateInputs,
    planDirectEditAdd: directEditWorkflow.actions.planDirectEditAdd,
    requestArchiveOpen: archiveSession.open.requestArchiveOpen,
    handleStartupRequest: archiveSession.startup.handleStartupRequest
  });
  const archiveView = useArchiveViewState({
    manifest,
    entryFilter: entryBrowser.entryFilter,
    entrySort: entryBrowser.entrySort,
    selectedPaths: entryBrowser.selectedPaths,
    pendingDeletePaths: directEditWorkflow.pending.pendingDeletePaths,
    pendingAddEntries: directEditWorkflow.pending.pendingAddEntries,
    pendingReplaceEntries: directEditWorkflow.pending.pendingReplaceEntries
  });

  return buildWorkbenchArchiveWorkflowModel({
    entryBrowser,
    archiveSession,
    archiveView,
    archiveOpenAccess,
    createWorkflow,
    directEditWorkflow,
    pendingChangeAccess,
    ui: {
      dropState,
      canAddDirectEdit
    }
  });
}
