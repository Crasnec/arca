import { FileArchive, FolderOpen } from "lucide-react";
import type { ArchiveManifest } from "../../shared/types";
import { formatBytes } from "../../shared/format";
import type { EntryTableActions, EntryTableView } from "./entry-table";
import { useI18n } from "../../i18n";
import styles from "./entry-workspace.module.css";

export type EntryTableBodyProps = {
  manifest: ArchiveManifest | null;
  table: EntryTableView;
  actions: Pick<EntryTableActions, "onSelectEntry" | "onOpenEntryContextMenu">;
};

function EntryIcon({ entryType }: { entryType: string }) {
  return entryType === "directory" ? (
    <FolderOpen size={16} aria-hidden="true" />
  ) : (
    <FileArchive size={16} aria-hidden="true" />
  );
}

export function EntryTableBody({ manifest, table, actions }: EntryTableBodyProps) {
  const { t } = useI18n();
  const {
    visibleEntryCount,
    filterActive,
    entries,
    pendingAddEntries,
    selectedPathSet,
    pendingReplaceEntries
  } = table;
  const { onSelectEntry, onOpenEntryContextMenu } = actions;

  return (
    <tbody>
      {!manifest && (
        <tr>
          <td colSpan={5} className={styles.emptyCell}>
            {t("entry.openArchive")}
          </td>
        </tr>
      )}
      {manifest && visibleEntryCount === 0 && (
        <tr>
          <td colSpan={5} className={styles.emptyCell}>
            {filterActive ? t("entry.noEntriesMatch") : t("entry.noEntries")}
          </td>
        </tr>
      )}
      {entries.map((row) => (
        <tr
          aria-selected={selectedPathSet.has(row.path)}
          className={
            selectedPathSet.has(row.path)
              ? `${styles.entryRow} ${styles.entrySelected}`
              : styles.entryRow
          }
          key={row.path}
          onClick={(event) => onSelectEntry(row.path, event)}
          onContextMenu={(event) => onOpenEntryContextMenu(row.path, event)}
        >
          <td>
            <span className={styles.entryName}>
              <EntryIcon entryType={row.entryType} />
              {row.path}
            </span>
          </td>
          <td>{row.entryType}</td>
          <td>{formatBytes(row.uncompressedSize)}</td>
          <td>{row.compressedSize === null ? "-" : formatBytes(row.compressedSize)}</td>
          <td>{row.encrypted ? t("entry.yes") : t("entry.no")}</td>
        </tr>
      ))}
      {pendingAddEntries.map((row) => (
        <tr className={styles.entryRow} key={`pending:${row.archivePath}`}>
          <td>
            <span className={styles.entryName}>
              <EntryIcon entryType={row.entryType} />
              {row.archivePath}
            </span>
          </td>
          <td>
            {pendingReplaceEntries.includes(row.archivePath)
              ? t("entry.pendingReplace")
              : t("entry.pendingAdd")}
          </td>
          <td>-</td>
          <td>-</td>
          <td>{t("entry.no")}</td>
        </tr>
      ))}
    </tbody>
  );
}
