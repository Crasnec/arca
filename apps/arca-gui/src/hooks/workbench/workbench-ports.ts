import type { OperationRunner } from "../../shared/types";
import type { useWorkbenchState } from "./workbench-state";

type WorkbenchState = ReturnType<typeof useWorkbenchState>;

type WorkbenchPortsInput = {
  archiveState: WorkbenchState["archive"];
  feedback: WorkbenchState["feedback"];
  prompts: WorkbenchState["prompts"];
  refs: WorkbenchState["refs"];
  withOperation: OperationRunner;
};

export function buildWorkbenchPorts({
  archiveState,
  feedback,
  prompts,
  refs,
  withOperation
}: WorkbenchPortsInput) {
  const { passwordAction, overwritePrompt, unsavedPrompt } = prompts;

  const operation = { withOperation };
  const feedbackActions = {
    setLoading: feedback.setLoading,
    setError: feedback.setError,
    setStatus: feedback.setStatus
  };
  const infoFeedback = {
    setError: feedback.setError,
    setStatus: feedback.setStatus
  };
  const statusFeedback = {
    setStatus: feedback.setStatus
  };
  const promptActions = {
    setPasswordAction: prompts.setPasswordAction,
    setOverwritePrompt: prompts.setOverwritePrompt,
    setUnsavedPrompt: prompts.setUnsavedPrompt
  };
  const promptDialogState = {
    passwordAction,
    overwritePrompt,
    unsavedPrompt,
    passwordInputRef: refs.passwordInputRef,
    ...promptActions,
    setCloseBlockedPrompt: prompts.setCloseBlockedPrompt
  };
  const archiveAccess = {
    manifest: archiveState.manifest,
    setManifest: archiveState.setManifest
  };

  return {
    operation,
    feedbackActions,
    infoFeedback,
    statusFeedback,
    promptActions,
    promptDialogState,
    archiveAccess
  };
}

