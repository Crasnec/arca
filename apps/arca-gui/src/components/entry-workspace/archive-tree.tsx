import { Archive, FolderOpen } from "lucide-react";
import type { ArchiveManifest } from "../../shared/types";
import { useI18n } from "../../i18n";
import styles from "./entry-workspace.module.css";

export type ArchiveTreeProps = {
  manifest: ArchiveManifest | null;
  treeRows: string[];
};

export function ArchiveTree({ manifest, treeRows }: ArchiveTreeProps) {
  const { t } = useI18n();
  return (
    <aside className={styles.tree} aria-label={t("entry.treeAria")}>
      <div className={`${styles.treeRow} ${styles.selected}`}>
        <Archive size={16} aria-hidden="true" />
        <span>{manifest?.archiveName ?? t("entry.noArchive")}</span>
      </div>
      {treeRows.map((row) => (
        <div className={styles.treeRow} key={row}>
          <FolderOpen size={16} aria-hidden="true" />
          <span>{row}</span>
        </div>
      ))}
    </aside>
  );
}
