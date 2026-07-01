import { ArrowDown, ArrowUp, ArrowUpDown } from "lucide-react";
import type { EntrySortColumn, EntrySortState } from "../../shared/types";
import { useI18n } from "../../i18n";
import styles from "./entry-workspace.module.css";

type EntrySortAria = "none" | "ascending" | "descending";

export type EntryTableHeaderProps = {
  sort: EntrySortState;
  onSort: (column: EntrySortColumn) => void;
};

function entrySortAria(entrySort: EntrySortState, column: EntrySortColumn): EntrySortAria {
  if (entrySort.column !== column) {
    return "none";
  }
  return entrySort.direction === "asc" ? "ascending" : "descending";
}

function SortHeader({
  column,
  label,
  sortLabel,
  entrySort,
  onSort
}: {
  column: EntrySortColumn;
  label: string;
  sortLabel: string;
  entrySort: EntrySortState;
  onSort: (column: EntrySortColumn) => void;
}) {
  const active = entrySort.column === column;
  const SortIcon = active ? (entrySort.direction === "asc" ? ArrowUp : ArrowDown) : ArrowUpDown;
  return (
    <button
      type="button"
      className={`${styles.columnSort}${active ? ` ${styles.columnSortActive}` : ""}`}
      title={sortLabel}
      aria-label={sortLabel}
      onClick={() => onSort(column)}
    >
      <span>{label}</span>
      <SortIcon size={13} aria-hidden="true" />
    </button>
  );
}

export function EntryTableHeader({ sort, onSort }: EntryTableHeaderProps) {
  const { t } = useI18n();
  const labels = {
    path: t("entry.name"),
    entryType: t("entry.type"),
    uncompressedSize: t("entry.size"),
    compressedSize: t("entry.packed"),
    encrypted: t("entry.encrypted")
  };
  return (
    <thead>
      <tr>
        <th aria-sort={entrySortAria(sort, "path")}>
          <SortHeader
            column="path"
            label={labels.path}
            sortLabel={t("entry.sortBy", { label: labels.path })}
            entrySort={sort}
            onSort={onSort}
          />
        </th>
        <th aria-sort={entrySortAria(sort, "entryType")}>
          <SortHeader
            column="entryType"
            label={labels.entryType}
            sortLabel={t("entry.sortBy", { label: labels.entryType })}
            entrySort={sort}
            onSort={onSort}
          />
        </th>
        <th aria-sort={entrySortAria(sort, "uncompressedSize")}>
          <SortHeader
            column="uncompressedSize"
            label={labels.uncompressedSize}
            sortLabel={t("entry.sortBy", { label: labels.uncompressedSize })}
            entrySort={sort}
            onSort={onSort}
          />
        </th>
        <th aria-sort={entrySortAria(sort, "compressedSize")}>
          <SortHeader
            column="compressedSize"
            label={labels.compressedSize}
            sortLabel={t("entry.sortBy", { label: labels.compressedSize })}
            entrySort={sort}
            onSort={onSort}
          />
        </th>
        <th aria-sort={entrySortAria(sort, "encrypted")}>
          <SortHeader
            column="encrypted"
            label={labels.encrypted}
            sortLabel={t("entry.sortBy", { label: labels.encrypted })}
            entrySort={sort}
            onSort={onSort}
          />
        </th>
      </tr>
    </thead>
  );
}
