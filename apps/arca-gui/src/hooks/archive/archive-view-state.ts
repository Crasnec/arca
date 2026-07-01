import React from "react";
import type {
  ArchiveManifest,
  DirectEditPlannedEntry,
  EntrySortState
} from "../../shared/types";
import {
  isPendingDeleted,
  listEntryMatchesFilter,
  plannedEntryMatchesFilter,
  sortListEntries
} from "../../shared/entry-list-utils";

type ArchiveViewStateInput = {
  manifest: ArchiveManifest | null;
  entryFilter: string;
  entrySort: EntrySortState;
  selectedPaths: string[];
  pendingDeletePaths: string[];
  pendingAddEntries: DirectEditPlannedEntry[];
  pendingReplaceEntries: string[];
};

export function useArchiveViewState({
  manifest,
  entryFilter,
  entrySort,
  selectedPaths,
  pendingDeletePaths,
  pendingAddEntries,
  pendingReplaceEntries
}: ArchiveViewStateInput) {
  const pendingDeleteSet = React.useMemo(() => new Set(pendingDeletePaths), [pendingDeletePaths]);
  const pendingReplaceSet = React.useMemo(
    () => new Set(pendingReplaceEntries),
    [pendingReplaceEntries]
  );
  const pendingFilter = React.useMemo(
    () => entryFilter.trim().toLocaleLowerCase(),
    [entryFilter]
  );

  const visibleBaseEntries = React.useMemo(() => {
    if (!manifest) {
      return [];
    }
    return manifest.entries.filter(
      (entry) =>
        !isPendingDeleted(entry.path, pendingDeleteSet) &&
        !isPendingDeleted(entry.path, pendingReplaceSet)
    );
  }, [manifest, pendingDeleteSet, pendingReplaceSet]);

  const visibleEntries = React.useMemo(() => {
    const filtered = pendingFilter
      ? visibleBaseEntries.filter((entry) => listEntryMatchesFilter(entry, pendingFilter))
      : visibleBaseEntries;
    return sortListEntries(filtered, entrySort);
  }, [entrySort, pendingFilter, visibleBaseEntries]);

  const visiblePendingAddEntries = React.useMemo(() => {
    if (!pendingFilter) {
      return pendingAddEntries;
    }
    return pendingAddEntries.filter((entry) => plannedEntryMatchesFilter(entry, pendingFilter));
  }, [pendingAddEntries, pendingFilter]);

  const treeRows = React.useMemo(() => {
    if (!manifest) {
      return [];
    }
    const roots = new Set<string>();
    for (const entry of visibleEntries) {
      const root = entry.path.replace(/\/$/, "").split("/")[0];
      if (root) {
        roots.add(root);
      }
    }
    return [...roots].sort((a, b) => a.localeCompare(b)).slice(0, 24);
  }, [manifest, visibleEntries]);

  const selectedPathSet = React.useMemo(() => new Set(selectedPaths), [selectedPaths]);
  const selectedEntries = React.useMemo(() => {
    if (!manifest) {
      return [];
    }
    const entriesByPath = new Map(manifest.entries.map((entry) => [entry.path, entry]));
    return selectedPaths.flatMap((path) => {
      const entry = entriesByPath.get(path);
      return entry ? [entry] : [];
    });
  }, [manifest, selectedPaths]);

  const selectedUncompressedSize = selectedEntries.reduce(
    (total, entry) => total + entry.uncompressedSize,
    0
  );
  const selectedCompressedSize = selectedEntries.some((entry) => entry.compressedSize === null)
    ? null
    : selectedEntries.reduce((total, entry) => total + (entry.compressedSize ?? 0), 0);
  const selectedEncryptedCount = selectedEntries.filter((entry) => entry.encrypted).length;
  const visibleEntryCount = visibleEntries.length + visiblePendingAddEntries.length;
  const filterableEntryCount = visibleBaseEntries.length + pendingAddEntries.length;
  const entryFilterActive = pendingFilter !== "";

  return {
    table: {
      visibleEntries,
      visiblePendingAddEntries,
      treeRows,
      selectedPathSet,
      visibleEntryCount,
      filterableEntryCount,
      entryFilterActive
    },
    selection: {
      selectedEntries,
      selectedUncompressedSize,
      selectedCompressedSize,
      selectedEncryptedCount
    }
  };
}
