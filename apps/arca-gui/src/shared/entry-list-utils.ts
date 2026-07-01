import type {
  DirectEditPlannedEntry,
  EntrySortColumn,
  EntrySortState,
  ListEntry
} from "./types";

export function listEntryMatchesFilter(entry: ListEntry, filter: string) {
  return (
    entry.path.toLocaleLowerCase().includes(filter) ||
    entry.entryType.toLocaleLowerCase().includes(filter) ||
    (entry.encrypted ? "encrypted yes" : "unencrypted no").includes(filter)
  );
}

export function plannedEntryMatchesFilter(entry: DirectEditPlannedEntry, filter: string) {
  return (
    entry.archivePath.toLocaleLowerCase().includes(filter) ||
    entry.entryType.toLocaleLowerCase().includes(filter)
  );
}

export function sortListEntries(entries: ListEntry[], sort: EntrySortState) {
  const direction = sort.direction === "asc" ? 1 : -1;
  return [...entries].sort((left, right) => {
    const primary = compareListEntries(left, right, sort.column);
    if (primary !== 0) {
      return primary * direction;
    }
    return compareText(left.path, right.path);
  });
}

function compareListEntries(left: ListEntry, right: ListEntry, column: EntrySortColumn) {
  if (column === "path") {
    return compareText(left.path, right.path);
  }
  if (column === "entryType") {
    return compareText(left.entryType, right.entryType);
  }
  if (column === "uncompressedSize") {
    return compareNumber(left.uncompressedSize, right.uncompressedSize);
  }
  if (column === "compressedSize") {
    return compareNullableNumber(left.compressedSize, right.compressedSize);
  }
  return compareNumber(Number(left.encrypted), Number(right.encrypted));
}

function compareText(left: string, right: string) {
  return left.localeCompare(right, undefined, { numeric: true, sensitivity: "base" });
}

function compareNumber(left: number, right: number) {
  return left === right ? 0 : left < right ? -1 : 1;
}

function compareNullableNumber(left: number | null, right: number | null) {
  if (left === null && right === null) {
    return 0;
  }
  if (left === null) {
    return 1;
  }
  if (right === null) {
    return -1;
  }
  return compareNumber(left, right);
}

export function isPendingDeleted(path: string, pendingDeleteSet: Set<string>) {
  const normalized = path.replace(/\/$/, "");
  for (const pending of pendingDeleteSet) {
    const normalizedPending = pending.replace(/\/$/, "");
    if (normalized === normalizedPending || normalized.startsWith(`${normalizedPending}/`)) {
      return true;
    }
  }
  return false;
}
