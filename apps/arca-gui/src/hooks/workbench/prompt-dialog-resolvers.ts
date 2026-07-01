import type React from "react";
import type {
  ArchiveManifest,
  OverwritePromptState,
  PasswordAction,
  StartupRequest,
  UnsavedPromptState
} from "../../shared/types";

type UnsavedPromptResolverInput = {
  unsavedPrompt: UnsavedPromptState | null;
  setUnsavedPrompt: React.Dispatch<React.SetStateAction<UnsavedPromptState | null>>;
  resetAllPendingChanges: () => void;
  setArchivePath: React.Dispatch<React.SetStateAction<string>>;
  openArchivePath: (path: string) => Promise<ArchiveManifest | null>;
  runStartupRequest: (request: StartupRequest) => Promise<void>;
  showCreateModal: () => void;
};

type OverwritePromptResolverInput = {
  overwritePrompt: OverwritePromptState | null;
  setOverwritePrompt: React.Dispatch<React.SetStateAction<OverwritePromptState | null>>;
  createArchive: (overwrite?: boolean) => void | Promise<void>;
  extractArchive: (password?: string, overwrite?: boolean) => void | Promise<void>;
  extractSelectedEntries: (password?: string, overwrite?: boolean) => void | Promise<void>;
};

type PasswordPromptResolverInput = {
  event: React.FormEvent;
  passwordAction: PasswordAction | null;
  passwordInputRef: React.RefObject<HTMLInputElement | null>;
  setPasswordAction: React.Dispatch<React.SetStateAction<PasswordAction | null>>;
  extractArchive: (password?: string, overwrite?: boolean) => void | Promise<void>;
  extractSelectedEntries: (password?: string, overwrite?: boolean) => void | Promise<void>;
  testArchive: (password?: string) => void | Promise<void>;
  testSelectedEntries: (password?: string) => void | Promise<void>;
};

export function confirmDiscardPendingChanges({
  unsavedPrompt,
  setUnsavedPrompt,
  resetAllPendingChanges,
  setArchivePath,
  openArchivePath,
  runStartupRequest,
  showCreateModal
}: UnsavedPromptResolverInput) {
  const action = unsavedPrompt?.action;
  setUnsavedPrompt(null);
  resetAllPendingChanges();
  if (action?.kind === "openArchive") {
    setArchivePath(action.path);
    void openArchivePath(action.path);
  } else if (action?.kind === "startupRequest") {
    void runStartupRequest(action.request);
  } else if (action?.kind === "newArchive") {
    showCreateModal();
  }
}

export function confirmOverwrite({
  overwritePrompt,
  setOverwritePrompt,
  createArchive,
  extractArchive,
  extractSelectedEntries
}: OverwritePromptResolverInput) {
  const action = overwritePrompt?.action;
  setOverwritePrompt(null);
  if (action === "extract") {
    void extractArchive(undefined, true);
  } else if (action === "extractSelection") {
    void extractSelectedEntries(undefined, true);
  } else if (action === "create") {
    void createArchive(true);
  }
}

export function submitPassword({
  event,
  passwordAction,
  passwordInputRef,
  setPasswordAction,
  extractArchive,
  extractSelectedEntries,
  testArchive,
  testSelectedEntries
}: PasswordPromptResolverInput) {
  event.preventDefault();
  const action = passwordAction;
  if (!action) {
    return;
  }
  const password = passwordInputRef.current?.value ?? "";
  if (passwordInputRef.current) {
    passwordInputRef.current.value = "";
  }
  setPasswordAction(null);
  if (action === "test") {
    void testArchive(password);
  } else if (action === "extract") {
    void extractArchive(password);
  } else if (action === "testSelection") {
    void testSelectedEntries(password);
  } else {
    void extractSelectedEntries(password);
  }
}

