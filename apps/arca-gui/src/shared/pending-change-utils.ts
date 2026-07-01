import { PENDING_HISTORY_LIMIT } from "./constants";
import type { DirectEditPlannedEntry, PendingChangesSnapshot } from "./types";
import { currentLocale, translate } from "../i18n/messages";

export function pendingChangesMessage(count: number) {
  return translate(currentLocale(), "pending.discardChanges", {
    count,
    plural: count === 1 ? "" : "s"
  });
}

export function clonePendingChangesSnapshot(
  snapshot: PendingChangesSnapshot
): PendingChangesSnapshot {
  return {
    deletePaths: [...snapshot.deletePaths],
    addInputs: [...snapshot.addInputs],
    addEntries: snapshot.addEntries.map((entry) => ({ ...entry })),
    replaceEntries: [...snapshot.replaceEntries]
  };
}

export function limitPendingHistory(stack: PendingChangesSnapshot[]) {
  return stack.slice(-PENDING_HISTORY_LIMIT);
}

export function samePendingChangesSnapshot(
  left: PendingChangesSnapshot,
  right: PendingChangesSnapshot
) {
  return (
    sameStringList(left.deletePaths, right.deletePaths) &&
    sameStringList(left.addInputs, right.addInputs) &&
    samePlannedEntries(left.addEntries, right.addEntries) &&
    sameStringList(left.replaceEntries, right.replaceEntries)
  );
}

export function sameStringList(left: string[], right: string[]) {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function samePlannedEntries(left: DirectEditPlannedEntry[], right: DirectEditPlannedEntry[]) {
  return (
    left.length === right.length &&
    left.every((entry, index) => {
      const other = right[index];
      return (
        other !== undefined &&
        entry.archivePath === other.archivePath && entry.entryType === other.entryType
      );
    })
  );
}
