import { listArchive } from "../../api/archive-commands";
import type { CommandError, PasswordAction } from "../../shared/types";
import { archiveManifestFullyValidated } from "../../shared/archive-validation";
import { isOverwritePromptError } from "../../shared/command-errors";
import type {
  ArchiveManifestPort,
  FeedbackPort,
  OperationPort,
  OverwritePromptPort,
  PasswordPromptPort
} from "../workflow-ports";

type ArchivePayloadRunnerInput = {
  pendingChanges: {
    hasPendingChanges: boolean;
  };
  archive: Pick<ArchiveManifestPort, "setManifest">;
  selection: {
    resetSelection: () => void;
  };
  operation: OperationPort;
  feedback: FeedbackPort;
  prompts: PasswordPromptPort & OverwritePromptPort;
};

type PayloadOperationInput = {
  startStatus: string;
  action: PasswordAction;
  failedStatus: string;
  run: () => Promise<void>;
};

export function createArchivePayloadRunner({
  pendingChanges: { hasPendingChanges },
  archive: { setManifest },
  selection: { resetSelection },
  operation: { withOperation },
  feedback: { setLoading, setError, setStatus },
  prompts: { setPasswordAction, setOverwritePrompt }
}: ArchivePayloadRunnerInput) {
  function markArchiveFullyValidated(path: string, reason: string) {
    setManifest((current) => {
      if (!current || current.archivePath !== path) {
        return current;
      }
      return archiveManifestFullyValidated(current, reason);
    });
  }

  async function refreshArchiveAfterValidation(path: string, reason: string) {
    if (hasPendingChanges) {
      markArchiveFullyValidated(path, reason);
      return true;
    }
    try {
      const value = await withOperation("Refresh archive", (operationId) =>
        listArchive(path, operationId)
      );
      const validated = archiveManifestFullyValidated(value, reason);
      setManifest((current) => (current?.archivePath === path ? validated : current));
      resetSelection();
      return true;
    } catch (caught) {
      const commandError = caught as CommandError;
      setError(commandError.message ?? String(caught));
      return false;
    }
  }

  function handleCommandError(
    caught: unknown,
    action: PasswordAction,
    failedStatus: string
  ) {
    const commandError = caught as CommandError;
    setError(commandError.message ?? String(caught));
    if (
      (action === "extract" || action === "extractSelection") &&
      isOverwritePromptError(commandError)
    ) {
      setOverwritePrompt({
        action: action === "extractSelection" ? action : "extract",
        message: commandError.message ?? ""
      });
      setStatus("Replace required");
      return;
    }
    if (commandError.code === "password") {
      setPasswordAction(action);
      setStatus("Password required");
      return;
    }
    setStatus(commandError.code ? `${failedStatus}: ${commandError.code}` : failedStatus);
  }

  async function runPayloadOperation({
    startStatus,
    action,
    failedStatus,
    run
  }: PayloadOperationInput) {
    setLoading(true);
    setError(null);
    setStatus(startStatus);
    try {
      await run();
    } catch (caught) {
      handleCommandError(caught, action, failedStatus);
    } finally {
      setLoading(false);
    }
  }

  return {
    refreshArchiveAfterValidation,
    runPayloadOperation
  };
}
