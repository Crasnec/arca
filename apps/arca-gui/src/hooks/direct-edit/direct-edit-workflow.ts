import { createDirectEditAddActions } from "./direct-edit-add-actions";
import { createDirectEditFeedbackActions } from "./direct-edit-feedback-actions";
import { createDirectEditPendingActions } from "./direct-edit-pending-actions";
import { createDirectEditRunner } from "./direct-edit-runner";
import { useDirectEditReplacePrompt } from "./direct-edit-replace-prompt";
import { buildDirectEditWorkflowModel } from "./direct-edit-workflow-model";
import { usePendingChanges } from "./pending-changes";
import type {
  ArchiveManifestPort,
  FeedbackPort,
  OperationPort
} from "../workflow-ports";

type DirectEditWorkflowInput = {
  archive: ArchiveManifestPort;
  selection: {
    selectedPaths: string[];
    resetSelection: () => void;
  };
  capability: {
    canAddDirectEdit: boolean;
  };
  operation: OperationPort;
  feedback: FeedbackPort;
};

export function useDirectEditWorkflow({
  archive: { manifest, setManifest },
  selection: { selectedPaths, resetSelection },
  capability: { canAddDirectEdit },
  operation,
  feedback
}: DirectEditWorkflowInput) {
  const pendingChanges = usePendingChanges();
  const {
    pendingDeletePaths,
    pendingAddInputs,
    pendingAddEntries,
    pendingReplaceEntries,
    hasPendingChanges
  } = pendingChanges;
  const feedbackActions = createDirectEditFeedbackActions({
    manifest,
    pendingChanges,
    feedback
  });

  const {
    directEditReplacePrompt,
    setDirectEditReplacePrompt,
    closeDirectEditReplacePrompt,
    skipReplacementAddition,
    confirmReplacementAddition,
    skipReplacementAdditions,
    confirmReplacementAdditions
  } = useDirectEditReplacePrompt({
    appendPendingAddPlan: feedbackActions.appendPendingAddPlan,
    setStatus: feedback.setStatus
  });
  const replacement = {
    directEditReplacePrompt,
    setDirectEditReplacePrompt,
    closeDirectEditReplacePrompt,
    skipReplacementAddition,
    confirmReplacementAddition,
    skipReplacementAdditions,
    confirmReplacementAdditions
  };
  const { runPlanDirectEditAdd, runSaveDirectEdit } = createDirectEditRunner({
    operation,
    feedback
  });

  const addActions = createDirectEditAddActions({
    manifest,
    capability: {
      canAddDirectEdit
    },
    pending: {
      pendingDeletePaths,
      pendingAddEntries
    },
    setDirectEditReplacePrompt,
    appendPendingAddPlan: feedbackActions.appendPendingAddPlan,
    reportDirectEditUnavailable: feedbackActions.reportDirectEditUnavailable,
    runPlanDirectEditAdd,
    feedback
  });
  const pendingActions = createDirectEditPendingActions({
    manifest,
    selectedPaths,
    pendingChanges,
    pending: {
      pendingDeletePaths,
      pendingAddInputs,
      pendingAddEntries,
      pendingReplaceEntries,
      hasPendingChanges
    },
    setDirectEditReplacePrompt,
    setManifest,
    resetSelection,
    reportDirectEditUnavailable: feedbackActions.reportDirectEditUnavailable,
    runSaveDirectEdit,
    setError: feedback.setError,
    setStatus: feedback.setStatus
  });

  return buildDirectEditWorkflowModel({
    pendingChanges,
    replacement,
    addActions,
    pendingActions,
    feedbackActions
  });
}
