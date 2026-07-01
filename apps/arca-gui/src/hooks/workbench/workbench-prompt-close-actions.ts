import type React from "react";
import type {
  CloseBlockedPromptState,
  OverwritePromptState,
  PasswordAction,
  UnsavedPromptState
} from "../../shared/types";

type WorkbenchPromptCloseActionsInput = {
  passwordInputRef: React.RefObject<HTMLInputElement | null>;
  setPasswordAction: React.Dispatch<React.SetStateAction<PasswordAction | null>>;
  setOverwritePrompt: React.Dispatch<React.SetStateAction<OverwritePromptState | null>>;
  setUnsavedPrompt: React.Dispatch<React.SetStateAction<UnsavedPromptState | null>>;
  setCloseBlockedPrompt: React.Dispatch<
    React.SetStateAction<CloseBlockedPromptState | null>
  >;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
};

export function createWorkbenchPromptCloseActions({
  passwordInputRef,
  setPasswordAction,
  setOverwritePrompt,
  setUnsavedPrompt,
  setCloseBlockedPrompt,
  setStatus
}: WorkbenchPromptCloseActionsInput) {
  function closePasswordPrompt() {
    if (passwordInputRef.current) {
      passwordInputRef.current.value = "";
    }
    setPasswordAction(null);
    setStatus("Ready");
  }

  function closeOverwritePrompt() {
    setOverwritePrompt(null);
    setStatus("Ready");
  }

  function closeUnsavedPrompt() {
    setUnsavedPrompt(null);
    setStatus("Ready");
  }

  function closeCloseBlockedPrompt() {
    setCloseBlockedPrompt(null);
    setStatus("Waiting for commit");
  }

  return {
    closePasswordPrompt,
    closeOverwritePrompt,
    closeUnsavedPrompt,
    closeCloseBlockedPrompt
  };
}
