import React from "react";
import type { DirectEditPlannedEntry, PendingChangesSnapshot } from "../../shared/types";
import { clonePendingChangesSnapshot } from "../../shared/pending-change-utils";
import {
  nextPendingAddPlan,
  nextPendingDeletePaths,
  nextRedoHistoryStep,
  nextUndoHistoryStep,
  nextUndoStack
} from "./pending-change-state-utils";

export function usePendingChanges() {
  const [pendingDeletePaths, setPendingDeletePaths] = React.useState<string[]>([]);
  const [pendingAddInputs, setPendingAddInputs] = React.useState<string[]>([]);
  const [pendingAddEntries, setPendingAddEntries] = React.useState<DirectEditPlannedEntry[]>([]);
  const [pendingReplaceEntries, setPendingReplaceEntries] = React.useState<string[]>([]);
  const [pendingUndoStack, setPendingUndoStack] = React.useState<PendingChangesSnapshot[]>([]);
  const [pendingRedoStack, setPendingRedoStack] = React.useState<PendingChangesSnapshot[]>([]);

  const pendingChangeCount = pendingDeletePaths.length + pendingAddEntries.length;
  const hasPendingChanges = pendingChangeCount > 0;

  const pendingChangesSnapshot = React.useCallback(
    (): PendingChangesSnapshot =>
      clonePendingChangesSnapshot({
        deletePaths: pendingDeletePaths,
        addInputs: pendingAddInputs,
        addEntries: pendingAddEntries,
        replaceEntries: pendingReplaceEntries
      }),
    [pendingAddEntries, pendingAddInputs, pendingDeletePaths, pendingReplaceEntries]
  );

  const restorePendingChanges = React.useCallback((snapshot: PendingChangesSnapshot) => {
    const restored = clonePendingChangesSnapshot(snapshot);
    setPendingDeletePaths(restored.deletePaths);
    setPendingAddInputs(restored.addInputs);
    setPendingAddEntries(restored.addEntries);
    setPendingReplaceEntries(restored.replaceEntries);
  }, []);

  const clearPendingHistory = React.useCallback(() => {
    setPendingUndoStack([]);
    setPendingRedoStack([]);
  }, []);

  const resetPendingChanges = React.useCallback(() => {
    setPendingDeletePaths([]);
    setPendingAddInputs([]);
    setPendingAddEntries([]);
    setPendingReplaceEntries([]);
  }, []);

  const resetAllPendingChanges = React.useCallback(() => {
    resetPendingChanges();
    clearPendingHistory();
  }, [clearPendingHistory, resetPendingChanges]);

  const recordPendingChanges = React.useCallback(() => {
    const snapshot = pendingChangesSnapshot();
    setPendingUndoStack((current) => nextUndoStack(current, snapshot));
    setPendingRedoStack([]);
  }, [pendingChangesSnapshot]);

  const appendPendingAddPlan = React.useCallback(
    (
      inputs: string[],
      additions: DirectEditPlannedEntry[],
      replacements: DirectEditPlannedEntry[]
    ) => {
      const next = nextPendingAddPlan({
        currentInputs: pendingAddInputs,
        currentEntries: pendingAddEntries,
        currentReplaceEntries: pendingReplaceEntries,
        inputs,
        additions,
        replacements
      });
      if (next.count === 0) {
        return 0;
      }
      recordPendingChanges();
      setPendingAddInputs(next.addInputs);
      setPendingAddEntries(next.addEntries);
      setPendingReplaceEntries(next.replaceEntries);
      return next.count;
    },
    [pendingAddEntries, pendingAddInputs, pendingReplaceEntries, recordPendingChanges]
  );

  const markPathsForDelete = React.useCallback(
    (paths: string[]) => {
      const next = nextPendingDeletePaths(pendingDeletePaths, paths);
      if (next.result !== "changed") {
        return next.result;
      }
      recordPendingChanges();
      setPendingDeletePaths(next.paths);
      return next.result;
    },
    [pendingDeletePaths, recordPendingChanges]
  );

  const undoPendingChangeSet = React.useCallback(() => {
    const step = nextUndoHistoryStep({
      undoStack: pendingUndoStack,
      redoStack: pendingRedoStack,
      current: pendingChangesSnapshot()
    });
    if (!step) {
      return false;
    }
    setPendingUndoStack(step.undoStack);
    setPendingRedoStack(step.redoStack);
    restorePendingChanges(step.restore);
    return true;
  }, [pendingChangesSnapshot, pendingRedoStack, pendingUndoStack, restorePendingChanges]);

  const redoPendingChangeSet = React.useCallback(() => {
    const step = nextRedoHistoryStep({
      undoStack: pendingUndoStack,
      redoStack: pendingRedoStack,
      current: pendingChangesSnapshot()
    });
    if (!step) {
      return false;
    }
    setPendingUndoStack(step.undoStack);
    setPendingRedoStack(step.redoStack);
    restorePendingChanges(step.restore);
    return true;
  }, [pendingChangesSnapshot, pendingRedoStack, pendingUndoStack, restorePendingChanges]);

  return {
    pendingDeletePaths,
    pendingAddInputs,
    pendingAddEntries,
    pendingReplaceEntries,
    pendingUndoStack,
    pendingRedoStack,
    pendingChangeCount,
    hasPendingChanges,
    appendPendingAddPlan,
    markPathsForDelete,
    resetPendingChanges,
    resetAllPendingChanges,
    clearPendingHistory,
    undoPendingChangeSet,
    redoPendingChangeSet
  };
}
