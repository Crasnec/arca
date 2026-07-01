import type { ArchiveManifest, ListEntry } from "../../shared/types";
import { formatBytes } from "../../shared/format";
import { basename } from "../../shared/path-utils";
import { useI18n } from "../../i18n";
import styles from "./modals.module.css";

export type ArchiveInfoModalProps = {
  open: boolean;
  manifest: ArchiveManifest | null;
  close: () => void;
};

export function ArchiveInfoModal({ open, manifest, close }: ArchiveInfoModalProps) {
  const { t } = useI18n();
  if (!open || !manifest) {
    return null;
  }

  return (
    <div className={styles.backdrop} role="presentation">
      <div
        className={`${styles.modal} ${styles.infoModal}`}
        role="dialog"
        aria-label={t("archiveInfo.aria")}
      >
        <div className={styles.title}>{manifest.archiveName}</div>
        <dl className={styles.infoGrid}>
          <dt>{t("info.format")}</dt>
          <dd>{manifest.formatKind.toUpperCase()}</dd>
          <dt>{t("info.files")}</dt>
          <dd>{manifest.entryCount}</dd>
          <dt>{t("info.size")}</dt>
          <dd>{formatBytes(manifest.totalUncompressedSize)}</dd>
          <dt>{t("info.packed")}</dt>
          <dd>
            {manifest.totalCompressedSize === null
              ? "-"
              : formatBytes(manifest.totalCompressedSize)}
          </dd>
          <dt>{t("info.encrypted")}</dt>
          <dd>
            {manifest.encryptedEntryCount === 0
              ? t("entry.no")
              : t("info.entries", { count: manifest.encryptedEntryCount })}
          </dd>
        </dl>
        <div className={styles.actions}>
          <button type="button" onClick={close}>
            {t("modal.ok")}
          </button>
        </div>
      </div>
    </div>
  );
}

export type EntryInfoModalProps = {
  open: boolean;
  entries: ListEntry[];
  uncompressedSize: number;
  compressedSize: number | null;
  encryptedCount: number;
  close: () => void;
};

export function EntryInfoModal({
  open,
  entries,
  uncompressedSize,
  compressedSize,
  encryptedCount,
  close
}: EntryInfoModalProps) {
  const { t } = useI18n();
  if (!open || entries.length === 0) {
    return null;
  }

  return (
    <div className={styles.backdrop} role="presentation">
      <div
        className={`${styles.modal} ${styles.infoModal}`}
        role="dialog"
        aria-label={t("entryInfo.aria")}
      >
        <div className={styles.title}>
          {entries.length === 1
            ? basename(entries[0].path)
            : t("info.selectedEntries", { count: entries.length })}
        </div>
        <dl className={styles.infoGrid}>
          {entries.length === 1 && (
            <>
              <dt>{t("info.path")}</dt>
              <dd>{entries[0].path}</dd>
            </>
          )}
          {entries.length > 1 && (
            <>
              <dt>{t("info.files")}</dt>
              <dd>{entries.length}</dd>
            </>
          )}
          <dt>{t("info.size")}</dt>
          <dd>{formatBytes(uncompressedSize)}</dd>
          <dt>{t("info.packed")}</dt>
          <dd>{compressedSize === null ? "-" : formatBytes(compressedSize)}</dd>
          <dt>{t("info.encrypted")}</dt>
          <dd>{encryptedCount === 0 ? t("entry.no") : t("info.entries", { count: encryptedCount })}</dd>
        </dl>
        <div className={styles.actions}>
          <button type="button" onClick={close}>
            {t("modal.ok")}
          </button>
        </div>
      </div>
    </div>
  );
}
