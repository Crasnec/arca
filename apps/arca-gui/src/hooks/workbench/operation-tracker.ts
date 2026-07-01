import React from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { OPERATION_PROGRESS_EVENT } from "../../shared/constants";
import type {
  CloseBlockedPromptState,
  CommandError,
  OperationProgress
} from "../../shared/types";

type OperationTrackerInput = {
  setStatus: React.Dispatch<React.SetStateAction<string>>;
  setError: React.Dispatch<React.SetStateAction<string | null>>;
  setCloseBlockedPrompt: React.Dispatch<
    React.SetStateAction<CloseBlockedPromptState | null>
  >;
};

export function useOperationTracker({
  setStatus,
  setError,
  setCloseBlockedPrompt
}: OperationTrackerInput) {
  const [activeOperation, setActiveOperation] =
    React.useState<OperationProgress | null>(null);

  React.useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | null = null;

    listen<OperationProgress>(OPERATION_PROGRESS_EVENT, (event) => {
      if (!mounted) {
        return;
      }
      const progress = event.payload;
      if (progress.phase === "cancelRequested") {
        setStatus("Cancel requested");
      }
      if (["finished", "failed", "canceled"].includes(progress.phase)) {
        setCloseBlockedPrompt(null);
      }
      setActiveOperation((current) => {
        if (current && current.id !== progress.id) {
          return current;
        }
        return progress;
      });
    })
      .then((value) => {
        unlisten = value;
      })
      .catch(() => {
        if (mounted) {
          setStatus("Ready");
        }
      });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [setCloseBlockedPrompt, setStatus]);

  const withOperation = React.useCallback(
    async <T,>(label: string, work: (operationId: number) => Promise<T>): Promise<T> => {
      const operationId = await invoke<number>("begin_operation", { label });
      setActiveOperation({
        id: operationId,
        label,
        phase: "started",
        message: "Operation queued",
        cancelRequested: false,
        cancellable: true,
        processed: null,
        total: null
      });
      try {
        return await work(operationId);
      } finally {
        try {
          await invoke("discard_operation", { operationId });
        } catch {
          // Archive commands claim and finish operation handles themselves; discard is best-effort.
        }
        setActiveOperation((current) => (current?.id === operationId ? null : current));
      }
    },
    []
  );

  const cancelActiveOperation = React.useCallback(async () => {
    const operation = activeOperation;
    if (!operation || operation.cancelRequested) {
      return;
    }
    setActiveOperation({
      ...operation,
      phase: "cancelRequested",
      message: "Cancel requested",
      cancelRequested: true,
      cancellable: false,
      processed: operation.processed,
      total: operation.total
    });
    setStatus("Cancel requested");
    try {
      await invoke("cancel_operation", { operationId: operation.id });
    } catch (caught) {
      const commandError = caught as CommandError;
      setError(commandError.message ?? String(caught));
      setStatus(commandError.code ? `Cancel failed: ${commandError.code}` : "Cancel failed");
    }
  }, [activeOperation, setError, setStatus]);

  return {
    activeOperation,
    withOperation,
    cancelActiveOperation
  };
}
