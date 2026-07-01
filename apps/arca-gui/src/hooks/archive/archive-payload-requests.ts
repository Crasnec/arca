import type React from "react";
import type { ArchiveManifest } from "../../shared/types";
import type {
  FeedbackPort,
  OperationPort,
  OverwritePromptPort,
  PasswordPromptPort
} from "../workflow-ports";
import { createArchivePayloadActions } from "./archive-payload-actions";
import { useArchiveExtractDestination } from "./archive-extract-destination";
import { createArchivePayloadExtractRequests } from "./archive-payload-extract-requests";
import { createArchivePayloadTestRequests } from "./archive-payload-test-requests";

type PayloadOperationRunner = Parameters<
  typeof createArchivePayloadActions
>[0]["runPayloadOperation"];

type ArchivePayloadRequestsInput = {
  archive: {
    destinationPath: string;
    manifest: ArchiveManifest | null;
    setDestinationPath: React.Dispatch<React.SetStateAction<string>>;
  };
  selection: {
    selectedPaths: string[];
  };
  operation: OperationPort;
  feedback: FeedbackPort;
  prompts: PasswordPromptPort & OverwritePromptPort;
  refreshArchiveAfterValidation: (path: string, reason: string) => Promise<boolean>;
  runPayloadOperation: PayloadOperationRunner;
};

export function useArchivePayloadRequests({
  archive: { destinationPath, manifest, setDestinationPath },
  selection: { selectedPaths },
  operation,
  feedback,
  prompts,
  refreshArchiveAfterValidation,
  runPayloadOperation
}: ArchivePayloadRequestsInput) {
  const {
    destinationPathRef,
    updateDestinationPath,
    chooseExtractDestination
  } = useArchiveExtractDestination({
    archive: {
      destinationPath,
      manifest,
      setDestinationPath
    },
    feedback
  });

  const {
    runArchiveTest,
    runArchiveExtract,
    runSelectedEntriesTest,
    runSelectedEntriesExtract
  } = createArchivePayloadActions({
    operation,
    feedback,
    prompts,
    refreshArchiveAfterValidation,
    runPayloadOperation
  });
  const testRequests = createArchivePayloadTestRequests({
    manifest,
    selectedPaths,
    feedback,
    actions: {
      runArchiveTest,
      runSelectedEntriesTest
    }
  });
  const extractRequests = createArchivePayloadExtractRequests({
    manifest,
    selectedPaths,
    destinationPathRef,
    updateDestinationPath,
    feedback,
    actions: {
      runArchiveExtract,
      runSelectedEntriesExtract
    }
  });

  return {
    ...testRequests,
    ...extractRequests,
    chooseExtractDestination,
    updateDestinationPath
  };
}
