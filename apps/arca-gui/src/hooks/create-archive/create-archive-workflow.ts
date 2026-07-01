import React from "react";
import { createCreateArchiveActions } from "./create-archive-actions";
import { createArchiveRunner } from "./create-archive-runner";
import { useCreateArchiveState } from "./create-archive-state";
import type { ArchiveManifest } from "../../shared/types";
import type {
  FeedbackPort,
  OperationPort,
  OverwritePromptPort,
  UnsavedPromptPort
} from "../workflow-ports";

type CreateArchiveWorkflowInput = {
  state: {
    loading: boolean;
  };
  archive: {
    setArchivePath: React.Dispatch<React.SetStateAction<string>>;
    openArchivePath: (path: string) => Promise<ArchiveManifest | null>;
  };
  pendingChanges: {
    hasPendingChanges: boolean;
    pendingChangeCount: number;
    resetAllPendingChanges: () => void;
  };
  operation: OperationPort;
  feedback: FeedbackPort;
  prompts: OverwritePromptPort & UnsavedPromptPort;
};

export function useCreateArchiveWorkflow({
  state: { loading },
  archive,
  pendingChanges,
  operation,
  feedback,
  prompts
}: CreateArchiveWorkflowInput) {
  const createArchiveState = useCreateArchiveState({ loading });
  const {
    createOpen,
    createOutputPath,
    createInputs,
    createEncrypt,
    createPasswordInputRef,
    createEncryptionAllowed,
    createSingleStreamOutput,
    canCreateArchive,
    setCreateOutputPath,
    setCreateEncrypt,
    removeCreateInput
  } = createArchiveState;
  const { runCreateArchiveCommand } = createArchiveRunner({
    operation,
    feedback,
    prompts
  });
  const createArchiveActions = createCreateArchiveActions({
    state: createArchiveState,
    archive,
    pendingChanges,
    feedback,
    prompts,
    runCreateArchiveCommand
  });

  return {
    state: {
      createOpen,
      createOutputPath,
      createInputs,
      createEncrypt,
      createPasswordInputRef,
      createEncryptionAllowed,
      createSingleStreamOutput,
      canCreateArchive
    },
    actions: {
      setCreateOutputPath,
      setCreateEncrypt,
      removeCreateInput,
      ...createArchiveActions
    }
  };
}
