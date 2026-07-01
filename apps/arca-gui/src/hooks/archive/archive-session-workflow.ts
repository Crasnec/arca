import type React from "react";
import { createArchiveSessionActions } from "./archive-session-actions";
import { createArchiveSessionRunner } from "./archive-session-runner";
import { useArchivePayloadWorkflow } from "./archive-payload-workflow";
import type {
  ArchiveManifestPort,
  FeedbackPort,
  OperationPort,
  OverwritePromptPort,
  PasswordPromptPort,
  UnsavedPromptPort
} from "../workflow-ports";

type ArchiveSessionWorkflowInput = {
  archive: ArchiveManifestPort & {
    path: string;
    destinationPath: string;
    setPath: React.Dispatch<React.SetStateAction<string>>;
    setDestinationPath: React.Dispatch<React.SetStateAction<string>>;
  };
  selection: {
    selectedPaths: string[];
    resetSelection: () => void;
    resetEntryBrowserState: () => void;
  };
  pendingChanges: {
    hasPendingChanges: boolean;
    pendingChangeCount: number;
    resetAllPendingChanges: () => void;
  };
  operation: OperationPort;
  feedback: FeedbackPort;
  prompts: PasswordPromptPort & OverwritePromptPort & UnsavedPromptPort;
};

export function useArchiveSessionWorkflow({
  archive: {
    path: archivePath,
    destinationPath,
    manifest,
    setPath: setArchivePath,
    setDestinationPath,
    setManifest
  },
  selection: { selectedPaths, resetSelection, resetEntryBrowserState },
  pendingChanges,
  operation,
  feedback,
  prompts
}: ArchiveSessionWorkflowInput) {
  const {
    runStartupTest,
    runStartupExtract,
    chooseExtractDestination,
    testArchive,
    testSelectedEntries,
    updateDestinationPath,
    extractArchive,
    extractSelectedEntries,
    extractSelectedEntriesHere
  } = useArchivePayloadWorkflow({
    archive: {
      destinationPath,
      manifest,
      setDestinationPath,
      setManifest
    },
    selection: {
      selectedPaths,
      resetSelection
    },
    pendingChanges,
    operation,
    feedback,
    prompts
  });
  const { openArchivePath } = createArchiveSessionRunner({
    archive: { setManifest },
    selection: { resetEntryBrowserState },
    pendingChanges,
    operation,
    feedback,
    prompts
  });
  const {
    openArchive,
    requestArchiveOpen,
    chooseArchive,
    handleStartupRequest,
    runStartupRequest
  } = createArchiveSessionActions({
    archive: {
      path: archivePath,
      setPath: setArchivePath
    },
    pendingChanges,
    feedback,
    prompts,
    openArchivePath,
    payload: {
      runStartupTest,
      runStartupExtract,
      updateDestinationPath
    }
  });

  return {
    open: {
      openArchivePath,
      openArchive,
      requestArchiveOpen,
      chooseArchive
    },
    startup: {
      handleStartupRequest,
      runStartupRequest
    },
    payload: {
      chooseExtractDestination,
      testArchive,
      testSelectedEntries,
      updateDestinationPath,
      extractArchive,
      extractSelectedEntries,
      extractSelectedEntriesHere
    }
  };
}
