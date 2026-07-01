import React from "react";
import type {
  ArchiveManifest,
  CloseBlockedPromptState,
  OverwritePromptState,
  PasswordAction,
  UnsavedPromptState
} from "../../shared/types";

export function useWorkbenchState() {
  const [archivePath, setArchivePath] = React.useState("");
  const [destinationPath, setDestinationPath] = React.useState("");
  const [manifest, setManifest] = React.useState<ArchiveManifest | null>(null);
  const [status, setStatus] = React.useState("Ready");
  const [, setError] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState(false);
  const [passwordAction, setPasswordAction] = React.useState<PasswordAction | null>(null);
  const [overwritePrompt, setOverwritePrompt] = React.useState<OverwritePromptState | null>(null);
  const [unsavedPrompt, setUnsavedPrompt] = React.useState<UnsavedPromptState | null>(null);
  const [closeBlockedPrompt, setCloseBlockedPrompt] =
    React.useState<CloseBlockedPromptState | null>(null);
  const passwordInputRef = React.useRef<HTMLInputElement | null>(null);

  return {
    archive: {
      archivePath,
      setArchivePath,
      destinationPath,
      setDestinationPath,
      manifest,
      setManifest
    },
    feedback: {
      status,
      setStatus,
      setError,
      loading,
      setLoading
    },
    prompts: {
      passwordAction,
      setPasswordAction,
      overwritePrompt,
      setOverwritePrompt,
      unsavedPrompt,
      setUnsavedPrompt,
      closeBlockedPrompt,
      setCloseBlockedPrompt
    },
    refs: {
      passwordInputRef
    }
  };
}
