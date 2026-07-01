import React from "react";
import type { ArchiveManifest, ListEntry } from "../../shared/types";

type EntrySelectionStateInput = {
  manifest: ArchiveManifest | null;
};

export function useEntrySelectionState({ manifest }: EntrySelectionStateInput) {
  const [selectedPaths, setSelectedPaths] = React.useState<string[]>([]);
  const [selectionAnchorPath, setSelectionAnchorPath] = React.useState<string | null>(null);

  function resetSelection() {
    setSelectedPaths([]);
    setSelectionAnchorPath(null);
  }

  function selectEntry(
    path: string,
    event: React.MouseEvent<HTMLTableRowElement>,
    visibleEntries: ListEntry[]
  ) {
    setSelectedPaths((current) => {
      if (event.shiftKey && manifest) {
        const paths = visibleEntries.map((entry) => entry.path);
        const anchor =
          selectionAnchorPath && paths.includes(selectionAnchorPath)
            ? selectionAnchorPath
            : current[0] ?? path;
        const anchorIndex = paths.indexOf(anchor);
        const pathIndex = paths.indexOf(path);
        if (anchorIndex !== -1 && pathIndex !== -1) {
          const start = Math.min(anchorIndex, pathIndex);
          const end = Math.max(anchorIndex, pathIndex);
          return paths.slice(start, end + 1);
        }
      }
      if (event.ctrlKey || event.metaKey) {
        return current.includes(path)
          ? current.filter((selected) => selected !== path)
          : [...current, path];
      }
      return current.length === 1 && current[0] === path ? current : [path];
    });
    if (!event.shiftKey) {
      setSelectionAnchorPath(path);
    }
  }

  function selectContextMenuPath(path: string) {
    const nextSelectedPaths = selectedPaths.includes(path) ? selectedPaths : [path];
    setSelectedPaths(nextSelectedPaths);
    setSelectionAnchorPath(path);
    return nextSelectedPaths;
  }

  return {
    selectedPaths,
    resetSelection,
    selectEntry,
    selectContextMenuPath
  };
}

