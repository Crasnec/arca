import { useOperationTracker } from "./operation-tracker";
import { useWorkbenchArchiveWorkflows } from "./workbench-archive-workflows";
import { useWorkbenchInteractions } from "./workbench-interactions";
import {
  buildWorkbenchInteractionInput,
  buildWorkbenchWorkflowModel
} from "./workbench-workflow-model";
import { buildWorkbenchPorts } from "./workbench-ports";
import { useWorkbenchState } from "./workbench-state";

export function useWorkbenchWorkflows() {
  const {
    archive: archiveState,
    feedback,
    prompts,
    refs
  } = useWorkbenchState();
  const { activeOperation, withOperation, cancelActiveOperation } = useOperationTracker({
    setStatus: feedback.setStatus,
    setError: feedback.setError,
    setCloseBlockedPrompt: prompts.setCloseBlockedPrompt
  });
  const {
    operation,
    feedbackActions,
    infoFeedback,
    statusFeedback,
    promptActions,
    promptDialogState,
    archiveAccess
  } = buildWorkbenchPorts({
    archiveState,
    feedback,
    prompts,
    refs,
    withOperation
  });
  const archiveWorkflows = useWorkbenchArchiveWorkflows({
    archiveState,
    feedback,
    prompts,
    ports: {
      operation,
      feedbackActions,
      promptActions,
      archiveAccess
    }
  });
  const interactions = useWorkbenchInteractions(
    buildWorkbenchInteractionInput({
      archiveState,
      feedback,
      prompts,
      ports: {
        infoFeedback,
        statusFeedback,
        promptDialogState
      },
      workflows: archiveWorkflows,
      operation: {
        cancelActiveOperation
      }
    })
  );

  return buildWorkbenchWorkflowModel({
    archiveState,
    feedback,
    prompts,
    refs,
    operation: {
      activeOperation
    },
    workflows: archiveWorkflows,
    interactions
  });
}
