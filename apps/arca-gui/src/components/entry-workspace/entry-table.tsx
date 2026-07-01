import React from "react";
import { EntryTableBody } from "./entry-table-body";
import { EntryTableHeader } from "./entry-table-header";
import type {
  ArchiveManifest,
  DirectEditPlannedEntry,
  EntrySortColumn,
  EntrySortState,
  ListEntry
} from "../../shared/types";
import { useI18n } from "../../i18n";
import styles from "./entry-workspace.module.css";

export type EntryTableView = {
  sort: EntrySortState;
  visibleEntryCount: number;
  filterActive: boolean;
  entries: ListEntry[];
  pendingAddEntries: DirectEditPlannedEntry[];
  selectedPathSet: Set<string>;
  pendingReplaceEntries: string[];
};

export type EntryTableActions = {
  onSort: (column: EntrySortColumn) => void;
  onSelectEntry: (path: string, event: React.MouseEvent<HTMLTableRowElement>) => void;
  onOpenEntryContextMenu: (
    path: string,
    event: React.MouseEvent<HTMLTableRowElement>
  ) => void;
};

export type EntryTableProps = {
  manifest: ArchiveManifest | null;
  table: EntryTableView;
  actions: EntryTableActions;
};

export function EntryTable({ manifest, table, actions }: EntryTableProps) {
  const { t } = useI18n();
  return (
    <section className={styles.tablePanel} aria-label={t("entry.entriesAria")}>
      <div className={styles.tableScroll}>
        <table className={styles.table}>
          <EntryTableHeader sort={table.sort} onSort={actions.onSort} />
          <EntryTableBody manifest={manifest} table={table} actions={actions} />
        </table>
      </div>
    </section>
  );
}
