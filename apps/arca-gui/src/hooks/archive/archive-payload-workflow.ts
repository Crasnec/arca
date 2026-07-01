import type React from "react";
import { createArchivePayloadRunner } from "./archive-payload-runner";
import { useArchivePayloadRequests } from "./archive-payload-requests";
import type {
  ArchiveManifestPort,
  FeedbackPort,
  OperationPort,
  OverwritePromptPort,
  PasswordPromptPort
} from "../workflow-ports";

type ArchivePayloadWorkflowInput = {
  archive: ArchiveManifestPort & {
    destinationPath: string;
    setDestinationPath: React.Dispatch<React.SetStateAction<string>>;
  };
  selection: {
    selectedPaths: string[];
    resetSelection: () => void;
  };
  pendingChanges: {
    hasPendingChanges: boolean;
  };
  operation: OperationPort;
  feedback: FeedbackPort;
  prompts: PasswordPromptPort & OverwritePromptPort;
};

export function useArchivePayloadWorkflow({
  archive: { destinationPath, manifest, setDestinationPath, setManifest },
  selection: { selectedPaths, resetSelection },
  pendingChanges,
  operation,
  feedback,
  prompts
}: ArchivePayloadWorkflowInput) {
  const { refreshArchiveAfterValidation, runPayloadOperation } =
    createArchivePayloadRunner({
      pendingChanges,
      archive: {
        setManifest
      },
      selection: {
        resetSelection
      },
      operation,
      feedback,
      prompts
    });

  return useArchivePayloadRequests({
    archive: {
      destinationPath,
      manifest,
      setDestinationPath
    },
    selection: {
      selectedPaths
    },
    operation,
    feedback,
    prompts,
    refreshArchiveAfterValidation,
    runPayloadOperation
  });
}
