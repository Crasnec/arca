import React from "react";
import type { ArchiveManifest, ListEntry } from "../../shared/types";

export type WorkbenchInfoDialogsInput = {
  manifest: ArchiveManifest | null;
  selectedEntries: ListEntry[];
  setError: React.Dispatch<React.SetStateAction<string | null>>;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
};

export function useWorkbenchInfoDialogs({
  manifest,
  selectedEntries,
  setError,
  setStatus
}: WorkbenchInfoDialogsInput) {
  const [archiveInfoOpen, setArchiveInfoOpen] = React.useState(false);
  const [entryInfoOpen, setEntryInfoOpen] = React.useState(false);

  function openArchiveInfo() {
    if (!manifest) {
      setError("Open an archive before viewing information");
      setStatus("Info unavailable");
      return;
    }
    setArchiveInfoOpen(true);
    setStatus("Archive information");
  }

  function closeArchiveInfo() {
    setArchiveInfoOpen(false);
    setStatus("Ready");
  }

  function openSelectedEntryInfo() {
    if (selectedEntries.length === 0) {
      setError("Select archive entries before opening properties");
      setStatus("Properties unavailable");
      return;
    }
    setEntryInfoOpen(true);
    setStatus(selectedEntries.length === 1 ? "Entry properties" : "Selection properties");
  }

  function closeSelectedEntryInfo() {
    setEntryInfoOpen(false);
    setStatus("Ready");
  }

  return {
    archiveInfo: {
      open: archiveInfoOpen,
      openDialog: openArchiveInfo,
      close: closeArchiveInfo
    },
    entryInfo: {
      open: entryInfoOpen,
      openDialog: openSelectedEntryInfo,
      close: closeSelectedEntryInfo
    }
  };
}
