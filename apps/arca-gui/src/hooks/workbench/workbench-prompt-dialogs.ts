import React from "react";
import type {
  ArchiveManifest,
  CloseBlockedPromptState,
  OverwritePromptState,
  PasswordAction,
  StartupRequest,
  UnsavedPromptState
} from "../../shared/types";
import { createWorkbenchPromptCloseActions } from "./workbench-prompt-close-actions";
import { buildWorkbenchPromptDialogModel } from "./workbench-prompt-dialog-model";

export type WorkbenchPromptDialogsInput = {
  prompts: {
    passwordAction: PasswordAction | null;
    overwritePrompt: OverwritePromptState | null;
    unsavedPrompt: UnsavedPromptState | null;
    passwordInputRef: React.RefObject<HTMLInputElement | null>;
    setPasswordAction: React.Dispatch<React.SetStateAction<PasswordAction | null>>;
    setOverwritePrompt: React.Dispatch<React.SetStateAction<OverwritePromptState | null>>;
    setUnsavedPrompt: React.Dispatch<React.SetStateAction<UnsavedPromptState | null>>;
    setCloseBlockedPrompt: React.Dispatch<
      React.SetStateAction<CloseBlockedPromptState | null>
    >;
  };
  archive: {
    setArchivePath: React.Dispatch<React.SetStateAction<string>>;
    openArchivePath: (path: string) => Promise<ArchiveManifest | null>;
  };
  pendingChanges: {
    resetAllPendingChanges: () => void;
  };
  startup: {
    runStartupRequest: (request: StartupRequest) => Promise<void>;
  };
  create: {
    showCreateModal: () => void;
    createArchive: (overwrite?: boolean) => void | Promise<void>;
  };
  archivePayload: {
    extractArchive: (password?: string, overwrite?: boolean) => void | Promise<void>;
    extractSelectedEntries: (password?: string, overwrite?: boolean) => void | Promise<void>;
    testArchive: (password?: string) => void | Promise<void>;
    testSelectedEntries: (password?: string) => void | Promise<void>;
  };
  feedback: {
    setStatus: React.Dispatch<React.SetStateAction<string>>;
  };
};

export function useWorkbenchPromptDialogs({
  prompts: {
    passwordAction,
    overwritePrompt,
    unsavedPrompt,
    passwordInputRef,
    setPasswordAction,
    setOverwritePrompt,
    setUnsavedPrompt,
    setCloseBlockedPrompt
  },
  archive: { setArchivePath, openArchivePath },
  pendingChanges: { resetAllPendingChanges },
  startup: { runStartupRequest },
  create: { showCreateModal, createArchive },
  archivePayload: {
    extractArchive,
    extractSelectedEntries,
    testArchive,
    testSelectedEntries
  },
  feedback: { setStatus }
}: WorkbenchPromptDialogsInput) {
  React.useEffect(() => {
    if (passwordAction) {
      passwordInputRef.current?.focus();
    }
  }, [passwordAction, passwordInputRef]);

  const closeActions = createWorkbenchPromptCloseActions({
    passwordInputRef,
    setPasswordAction,
    setOverwritePrompt,
    setUnsavedPrompt,
    setCloseBlockedPrompt,
    setStatus
  });

  return buildWorkbenchPromptDialogModel({
    prompts: {
      passwordAction,
      overwritePrompt,
      unsavedPrompt,
      passwordInputRef,
      setPasswordAction,
      setOverwritePrompt,
      setUnsavedPrompt
    },
    archive: {
      setArchivePath,
      openArchivePath
    },
    pendingChanges: {
      resetAllPendingChanges
    },
    startup: {
      runStartupRequest
    },
    create: {
      showCreateModal,
      createArchive
    },
    archivePayload: {
      extractArchive,
      extractSelectedEntries,
      testArchive,
      testSelectedEntries
    },
    closeActions
  });
}
