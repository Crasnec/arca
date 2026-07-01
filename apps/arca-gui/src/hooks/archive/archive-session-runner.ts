import { listArchive } from "../../api/archive-commands";
import type { ArchiveManifest, CommandError } from "../../shared/types";
import type {
  ArchiveManifestPort,
  FeedbackPort,
  OperationPort,
  OverwritePromptPort,
  UnsavedPromptPort
} from "../workflow-ports";

type ArchiveSessionRunnerInput = {
  archive: Pick<ArchiveManifestPort, "setManifest">;
  selection: {
    resetEntryBrowserState: () => void;
  };
  pendingChanges: {
    resetAllPendingChanges: () => void;
  };
  operation: OperationPort;
  feedback: FeedbackPort;
  prompts: OverwritePromptPort & UnsavedPromptPort;
};

export function createArchiveSessionRunner({
  archive: { setManifest },
  selection: { resetEntryBrowserState },
  pendingChanges: { resetAllPendingChanges },
  operation: { withOperation },
  feedback: { setLoading, setError, setStatus },
  prompts: { setOverwritePrompt, setUnsavedPrompt }
}: ArchiveSessionRunnerInput) {
  async function openArchivePath(path: string): Promise<ArchiveManifest | null> {
    if (!path) {
      setError("Archive path is required");
      setStatus("Open failed");
      return null;
    }

    setLoading(true);
    setError(null);
    setStatus("Opening archive");
    try {
      const value = await withOperation("Open archive", (operationId) =>
        listArchive(path, operationId)
      );
      setManifest(value);
      resetEntryBrowserState();
      resetAllPendingChanges();
      setOverwritePrompt(null);
      setUnsavedPrompt(null);
      setStatus(`Opened ${value.archiveName}`);
      return value;
    } catch (caught) {
      const commandError = caught as CommandError;
      setManifest(null);
      resetEntryBrowserState();
      resetAllPendingChanges();
      setOverwritePrompt(null);
      setUnsavedPrompt(null);
      setError(commandError.message ?? String(caught));
      setStatus(commandError.code ? `Open failed: ${commandError.code}` : "Open failed");
      return null;
    } finally {
      setLoading(false);
    }
  }

  return {
    openArchivePath
  };
}
