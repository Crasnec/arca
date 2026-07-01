import type React from "react";
import type {
  ArchiveManifest,
  OverwritePromptState,
  PasswordAction,
  StartupRequest,
  UnsavedPromptState
} from "../../shared/types";
import {
  confirmDiscardPendingChanges,
  confirmOverwrite,
  submitPassword
} from "./prompt-dialog-resolvers";
import type { createWorkbenchPromptCloseActions } from "./workbench-prompt-close-actions";

type PromptCloseActions = ReturnType<typeof createWorkbenchPromptCloseActions>;

type WorkbenchPromptDialogModelInput = {
  prompts: {
    passwordAction: PasswordAction | null;
    overwritePrompt: OverwritePromptState | null;
    unsavedPrompt: UnsavedPromptState | null;
    passwordInputRef: React.RefObject<HTMLInputElement | null>;
    setPasswordAction: React.Dispatch<React.SetStateAction<PasswordAction | null>>;
    setOverwritePrompt: React.Dispatch<React.SetStateAction<OverwritePromptState | null>>;
    setUnsavedPrompt: React.Dispatch<React.SetStateAction<UnsavedPromptState | null>>;
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
  closeActions: PromptCloseActions;
};

export function buildWorkbenchPromptDialogModel({
  prompts: {
    passwordAction,
    overwritePrompt,
    unsavedPrompt,
    passwordInputRef,
    setPasswordAction,
    setOverwritePrompt,
    setUnsavedPrompt
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
  closeActions
}: WorkbenchPromptDialogModelInput) {
  return {
    overwrite: {
      close: closeActions.closeOverwritePrompt,
      confirm: () =>
        confirmOverwrite({
          overwritePrompt,
          setOverwritePrompt,
          createArchive,
          extractArchive,
          extractSelectedEntries
        })
    },
    unsaved: {
      close: closeActions.closeUnsavedPrompt,
      discard: () =>
        confirmDiscardPendingChanges({
          unsavedPrompt,
          setUnsavedPrompt,
          resetAllPendingChanges,
          setArchivePath,
          openArchivePath,
          runStartupRequest,
          showCreateModal
        })
    },
    closeBlocked: {
      close: closeActions.closeCloseBlockedPrompt
    },
    password: {
      close: closeActions.closePasswordPrompt,
      submit: (event: React.FormEvent) =>
        submitPassword({
          event,
          passwordAction,
          passwordInputRef,
          setPasswordAction,
          extractArchive,
          extractSelectedEntries,
          testArchive,
          testSelectedEntries
        })
    }
  };
}
