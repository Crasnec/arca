import type { WorkbenchLayoutProps } from "../../components/workbench";
import type { AddressBarModelInput } from "./types";

export function buildAddressBarModel({
  archiveState: { archivePath, setArchivePath },
  feedback: { loading },
  browser: {
    entryFilter,
    entryFilterRef,
    updateEntryFilter,
    clearEntryFilter
  },
  archive: { open, table }
}: AddressBarModelInput): WorkbenchLayoutProps["addressBar"] {
  return {
    archive: { path: archivePath, loading },
    filter: {
      value: entryFilter,
      active: table.entryFilterActive,
      inputRef: entryFilterRef
    },
    actions: {
      onSubmit: open.openArchive,
      onChooseArchive: open.chooseArchive,
      onArchivePathChange: (value) => setArchivePath(value),
      onEntryFilterChange: updateEntryFilter,
      onClearEntryFilter: clearEntryFilter
    }
  };
}
