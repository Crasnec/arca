import React from "react";
import { CircleX, FolderOpen, Search } from "lucide-react";
import { useI18n } from "../i18n";
import styles from "./address-bar.module.css";

export type AddressBarProps = {
  archive: {
    path: string;
    loading: boolean;
  };
  filter: {
    value: string;
    active: boolean;
    inputRef: React.RefObject<HTMLInputElement | null>;
  };
  actions: {
    onSubmit: (event: React.FormEvent) => void;
    onChooseArchive: () => void | Promise<void>;
    onArchivePathChange: (value: string) => void;
    onEntryFilterChange: (value: string) => void;
    onClearEntryFilter: () => void;
  };
};

export function AddressBar({ archive, filter, actions }: AddressBarProps) {
  const { t } = useI18n();
  const {
    onSubmit,
    onChooseArchive,
    onArchivePathChange,
    onEntryFilterChange,
    onClearEntryFilter
  } = actions;
  return (
    <form className={styles.bar} aria-label={t("address.aria")} onSubmit={onSubmit}>
      <button
        type="button"
        className={styles.openButton}
        title={t("address.openArchive")}
        aria-label={t("address.openArchive")}
        disabled={archive.loading}
        onClick={() => void onChooseArchive()}
      >
        <FolderOpen size={17} aria-hidden="true" />
      </button>
      <input
        aria-label={t("address.archivePath")}
        className={styles.pathInput}
        value={archive.path}
        onChange={(event) => onArchivePathChange(event.target.value)}
        placeholder={t("address.archivePath")}
        spellCheck={false}
      />
      <label className={styles.filter}>
        <Search size={15} aria-hidden="true" />
        <input
          ref={filter.inputRef}
          aria-label={t("address.filterEntries")}
          value={filter.value}
          onChange={(event) => onEntryFilterChange(event.target.value)}
          placeholder={t("address.filter")}
          spellCheck={false}
        />
        {filter.active && (
          <button
            type="button"
            className={styles.filterClear}
            title={t("address.clearFilter")}
            aria-label={t("address.clearEntryFilter")}
            onClick={onClearEntryFilter}
          >
            <CircleX size={15} aria-hidden="true" />
          </button>
        )}
      </label>
    </form>
  );
}
