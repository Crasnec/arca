import type React from "react";
import { useCloseBlockedPrompt, useStartupRequests } from "./tauri-app-events";
import { useWorkbenchDragDrop } from "./workbench-drag-drop";
import type { CloseBlockedPromptState, DropState, StartupRequest } from "../../shared/types";

type WorkbenchEventsInput = {
  createOpen: boolean;
  directEditAllowed: boolean;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
  setCloseBlockedPrompt: React.Dispatch<
    React.SetStateAction<CloseBlockedPromptState | null>
  >;
  appendCreateInputs: (selected: string | string[] | null) => void;
  planDirectEditAdd: (selected: string | string[] | null) => void | Promise<void>;
  requestArchiveOpen: (path: string) => void;
  handleStartupRequest: (request: StartupRequest) => void | Promise<void>;
};

export function useWorkbenchEvents({
  createOpen,
  directEditAllowed,
  setStatus,
  setCloseBlockedPrompt,
  appendCreateInputs,
  planDirectEditAdd,
  requestArchiveOpen,
  handleStartupRequest
}: WorkbenchEventsInput): DropState {
  const dropState = useWorkbenchDragDrop({
    createOpen,
    directEditAllowed,
    setStatus,
    appendCreateInputs,
    planDirectEditAdd,
    requestArchiveOpen
  });

  useStartupRequests({
    onStartupRequest: handleStartupRequest,
    setStatus
  });

  useCloseBlockedPrompt({
    setCloseBlockedPrompt,
    setStatus
  });

  return dropState;
}
