import type { DirectEditPlannedEntry, PendingChangesSnapshot } from "../../shared/types";
import {
  limitPendingHistory,
  samePendingChangesSnapshot,
  sameStringList
} from "../../shared/pending-change-utils";

export type DeleteChangeResult = "empty" | "unchanged" | "changed";

type AppendPendingAddPlanInput = {
  currentInputs: string[];
  currentEntries: DirectEditPlannedEntry[];
  currentReplaceEntries: string[];
  inputs: string[];
  additions: DirectEditPlannedEntry[];
  replacements: DirectEditPlannedEntry[];
};

type PendingHistoryStepInput = {
  undoStack: PendingChangesSnapshot[];
  redoStack: PendingChangesSnapshot[];
  current: PendingChangesSnapshot;
};

export function nextPendingAddPlan({
  currentInputs,
  currentEntries,
  currentReplaceEntries,
  inputs,
  additions,
  replacements
}: AppendPendingAddPlanInput) {
  const entries = [...additions, ...replacements];
  return {
    count: entries.length,
    addInputs: [...new Set([...currentInputs, ...inputs])],
    addEntries: [...currentEntries, ...entries],
    replaceEntries: [
      ...new Set([...currentReplaceEntries, ...replacements.map((entry) => entry.archivePath)])
    ]
  };
}

export function nextPendingDeletePaths(current: string[], paths: string[]) {
  if (paths.length === 0) {
    return { result: "empty" as DeleteChangeResult, paths: current };
  }
  const next = [...new Set([...current, ...paths])];
  if (sameStringList(next, current)) {
    return { result: "unchanged" as DeleteChangeResult, paths: current };
  }
  return { result: "changed" as DeleteChangeResult, paths: next };
}

export function nextUndoStack(
  stack: PendingChangesSnapshot[],
  snapshot: PendingChangesSnapshot
) {
  const previous = stack[stack.length - 1];
  if (previous && samePendingChangesSnapshot(previous, snapshot)) {
    return stack;
  }
  return limitPendingHistory([...stack, snapshot]);
}

export function nextUndoHistoryStep({
  undoStack,
  redoStack,
  current
}: PendingHistoryStepInput) {
  const previous = undoStack[undoStack.length - 1];
  if (!previous) {
    return null;
  }
  return {
    restore: previous,
    undoStack: undoStack.slice(0, -1),
    redoStack: limitPendingHistory([...redoStack, current])
  };
}

export function nextRedoHistoryStep({
  undoStack,
  redoStack,
  current
}: PendingHistoryStepInput) {
  const next = redoStack[redoStack.length - 1];
  if (!next) {
    return null;
  }
  return {
    restore: next,
    undoStack: limitPendingHistory([...undoStack, current]),
    redoStack: redoStack.slice(0, -1)
  };
}
