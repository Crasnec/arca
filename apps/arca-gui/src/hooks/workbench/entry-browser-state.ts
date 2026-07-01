import React from "react";
import { showNativeEntryContextMenu } from "./native-menu-actions";
import { useEntrySelectionState } from "./entry-selection-state";
import { DEFAULT_ENTRY_SORT } from "../../shared/constants";
import type {
  ArchiveManifest,
  CommandError,
  EntrySortColumn,
  EntrySortState
} from "../../shared/types";

type EntryBrowserStateInput = {
  manifest: ArchiveManifest | null;
  canAddDirectEdit: boolean;
  loading: boolean;
  setError: React.Dispatch<React.SetStateAction<string | null>>;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
};

export function useEntryBrowserState({
  manifest,
  canAddDirectEdit,
  loading,
  setError,
  setStatus
}: EntryBrowserStateInput) {
  const [entryFilter, setEntryFilter] = React.useState("");
  const [entrySort, setEntrySort] = React.useState<EntrySortState>(DEFAULT_ENTRY_SORT);
  const entryFilterRef = React.useRef<HTMLInputElement | null>(null);
  const { selectedPaths, resetSelection, selectEntry, selectContextMenuPath } =
    useEntrySelectionState({ manifest });

  function resetEntryBrowserState() {
    setEntryFilter("");
    setEntrySort(DEFAULT_ENTRY_SORT);
    resetSelection();
  }

  function updateEntryFilter(value: string) {
    setEntryFilter(value);
    resetSelection();
  }

  function clearEntryFilter() {
    updateEntryFilter("");
    entryFilterRef.current?.focus();
  }

  function focusEntryFilter() {
    entryFilterRef.current?.focus();
    entryFilterRef.current?.select();
  }

  function changeEntrySort(column: EntrySortColumn) {
    setEntrySort((current) =>
      current.column === column
        ? {
            column,
            direction: current.direction === "asc" ? "desc" : "asc"
          }
        : {
            column,
            direction: "asc"
          }
    );
  }

  function openEntryContextMenu(path: string, event: React.MouseEvent<HTMLTableRowElement>) {
    event.preventDefault();
    const nextSelectedPaths = selectContextMenuPath(path);
    void showNativeEntryContextMenu({
      x: event.clientX,
      y: event.clientY,
      selectedPaths: nextSelectedPaths,
      manifest,
      canAddDirectEdit,
      loading
    }).catch((caught) => {
      const commandError = caught as CommandError;
      setError(commandError.message ?? String(caught));
      setStatus(
        commandError.code ? `Context menu failed: ${commandError.code}` : "Context menu failed"
      );
    });
  }

  async function copySelectedPaths() {
    if (selectedPaths.length === 0) {
      setError("Select archive entries before copying");
      setStatus("Copy failed");
      return;
    }

    try {
      const clipboard = navigator.clipboard;
      if (!clipboard?.writeText) {
        throw new Error("clipboard API is unavailable");
      }
      await clipboard.writeText(selectedPaths.join("\n"));
      setError(null);
      setStatus(
        selectedPaths.length === 1 ? "Path copied" : `${selectedPaths.length} paths copied`
      );
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
      setStatus("Copy failed");
    }
  }

  return {
    entryFilter,
    entrySort,
    selectedPaths,
    entryFilterRef,
    resetSelection,
    resetEntryBrowserState,
    selectEntry,
    updateEntryFilter,
    clearEntryFilter,
    focusEntryFilter,
    changeEntrySort,
    openEntryContextMenu,
    copySelectedPaths
  };
}
