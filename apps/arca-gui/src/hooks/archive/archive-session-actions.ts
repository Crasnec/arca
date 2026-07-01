import type React from "react";
import { chooseArchiveFile } from "../../api/file-dialogs";
import { pendingChangesMessage } from "../../shared/pending-change-utils";
import type { ArchiveManifest, StartupRequest } from "../../shared/types";
import type { FeedbackPort, UnsavedPromptPort } from "../workflow-ports";

type OpenArchivePath = (path: string) => Promise<ArchiveManifest | null>;

type StartupPayloadActions = {
  runStartupTest: (opened: ArchiveManifest) => Promise<void>;
  runStartupExtract: (opened: ArchiveManifest) => Promise<void>;
  updateDestinationPath: (path: string) => void;
};

type ArchiveSessionActionsInput = {
  archive: {
    path: string;
    setPath: React.Dispatch<React.SetStateAction<string>>;
  };
  pendingChanges: {
    hasPendingChanges: boolean;
    pendingChangeCount: number;
  };
  feedback: Pick<FeedbackPort, "setError" | "setStatus">;
  prompts: UnsavedPromptPort;
  openArchivePath: OpenArchivePath;
  payload: StartupPayloadActions;
};

export function createArchiveSessionActions({
  archive: { path: archivePath, setPath: setArchivePath },
  pendingChanges: { hasPendingChanges, pendingChangeCount },
  feedback: { setError, setStatus },
  prompts: { setUnsavedPrompt },
  openArchivePath,
  payload: { runStartupTest, runStartupExtract, updateDestinationPath }
}: ArchiveSessionActionsInput) {
  async function handleStartupRequest(request: StartupRequest) {
    if (hasPendingChanges) {
      setUnsavedPrompt({
        action: { kind: "startupRequest", request },
        message: pendingChangesMessage(pendingChangeCount)
      });
      setStatus("Unsaved changes");
      return;
    }
    await runStartupRequest(request);
  }

  async function runStartupRequest(request: StartupRequest) {
    setArchivePath(request.archivePath);
    updateDestinationPath("");
    const opened = await openArchivePath(request.archivePath);
    if (!opened || request.action === "open") {
      return;
    }

    if (request.action === "test") {
      await runStartupTest(opened);
    } else {
      await runStartupExtract(opened);
    }
  }

  function openArchive(event?: React.FormEvent) {
    event?.preventDefault();
    requestArchiveOpen(archivePath);
  }

  function requestArchiveOpen(path: string) {
    if (hasPendingChanges) {
      setUnsavedPrompt({
        action: { kind: "openArchive", path },
        message: pendingChangesMessage(pendingChangeCount)
      });
      setStatus("Unsaved changes");
      return;
    }
    setArchivePath(path);
    void openArchivePath(path);
  }

  async function chooseArchive() {
    setError(null);
    setStatus("Choosing archive");
    try {
      const selected = await chooseArchiveFile(archivePath);
      if (!selected) {
        setStatus("Open cancelled");
        return;
      }
      requestArchiveOpen(selected);
    } catch (caught) {
      setError(String(caught));
      setStatus("Open dialog failed");
    }
  }

  return {
    openArchive,
    requestArchiveOpen,
    chooseArchive,
    handleStartupRequest,
    runStartupRequest
  };
}
