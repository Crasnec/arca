import type React from "react";
import type {
  ArchiveManifest,
  OperationRunner,
  OverwritePromptState,
  PasswordAction,
  UnsavedPromptState
} from "../shared/types";

export type OperationPort = {
  withOperation: OperationRunner;
};

export type FeedbackPort = {
  setLoading: React.Dispatch<React.SetStateAction<boolean>>;
  setError: React.Dispatch<React.SetStateAction<string | null>>;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
};

export type ArchiveManifestPort = {
  manifest: ArchiveManifest | null;
  setManifest: React.Dispatch<React.SetStateAction<ArchiveManifest | null>>;
};

export type PasswordPromptPort = {
  setPasswordAction: React.Dispatch<React.SetStateAction<PasswordAction | null>>;
};

export type OverwritePromptPort = {
  setOverwritePrompt: React.Dispatch<React.SetStateAction<OverwritePromptState | null>>;
};

export type UnsavedPromptPort = {
  setUnsavedPrompt: React.Dispatch<React.SetStateAction<UnsavedPromptState | null>>;
};

