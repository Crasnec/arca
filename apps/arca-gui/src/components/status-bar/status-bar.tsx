import type { OperationProgress } from "../../shared/types";
import { useI18n } from "../../i18n";
import {
  type ArchiveStatusState,
  ArchiveMetrics,
  ArchiveValidationStatus,
  OperationStatus,
  StatusMessage,
  type StatusSummaryState,
  statusSummary
} from "./status-bar-sections";
import styles from "./status-bar.module.css";

export type StatusBarProps = {
  operation: OperationProgress | null;
  summary: StatusSummaryState;
  archive: ArchiveStatusState;
  message: string;
};

export function StatusBar({
  operation,
  summary,
  archive,
  message
}: StatusBarProps) {
  const { t } = useI18n();
  return (
    <footer className={styles.statusBar}>
      {operation ? (
        <OperationStatus operation={operation} />
      ) : (
        <span>{statusSummary(summary, t)}</span>
      )}
      <ArchiveMetrics manifest={archive.manifest} />
      <ArchiveValidationStatus archive={archive} />
      <StatusMessage message={message} />
    </footer>
  );
}
