import React from "react";
import { getCurrentWebview, type DragDropEvent } from "@tauri-apps/api/webview";
import type { DropState } from "../../shared/types";
import {
  describeCreateDropIntent,
  describeDirectEditDropIntent,
  describeDropIntent
} from "../../shared/drop-intents";

type WorkbenchDragDropInput = {
  createOpen: boolean;
  directEditAllowed: boolean;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
  appendCreateInputs: (selected: string | string[] | null) => void;
  planDirectEditAdd: (selected: string | string[] | null) => void | Promise<void>;
  requestArchiveOpen: (path: string) => void;
};

export function useWorkbenchDragDrop({
  createOpen,
  directEditAllowed,
  setStatus,
  appendCreateInputs,
  planDirectEditAdd,
  requestArchiveOpen
}: WorkbenchDragDropInput) {
  const [dropState, setDropState] = React.useState<DropState>("idle");

  const handleDragDropEvent = React.useCallback(
    (payload: DragDropEvent) => {
      if (payload.type === "enter" || payload.type === "over") {
        setDropState("hover");
        if (payload.type === "enter") {
          setStatus(
            createOpen
              ? describeCreateDropIntent(payload.paths)
              : directEditAllowed
                ? describeDirectEditDropIntent(payload.paths)
                : describeDropIntent(payload.paths)
          );
        }
        return;
      }

      setDropState("idle");
      if (payload.type !== "drop") {
        return;
      }

      const firstPath = payload.paths[0];
      if (!firstPath) {
        setStatus("Drop cancelled");
        return;
      }

      if (createOpen) {
        appendCreateInputs(payload.paths);
        return;
      }

      if (directEditAllowed) {
        void planDirectEditAdd(payload.paths);
        return;
      }

      requestArchiveOpen(firstPath);
    },
    [
      appendCreateInputs,
      createOpen,
      directEditAllowed,
      planDirectEditAdd,
      requestArchiveOpen,
      setStatus
    ]
  );

  React.useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | null = null;
    const tauriInternals = (
      window as Window & { __TAURI_INTERNALS__?: { metadata?: unknown } }
    ).__TAURI_INTERNALS__;
    if (!tauriInternals?.metadata) {
      return () => {
        mounted = false;
      };
    }

    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (!mounted) {
          return;
        }
        handleDragDropEvent(event.payload);
      })
      .then((value) => {
        unlisten = value;
      })
      .catch(() => {
        if (mounted) {
          setDropState("idle");
        }
      });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [handleDragDropEvent]);

  return dropState;
}
