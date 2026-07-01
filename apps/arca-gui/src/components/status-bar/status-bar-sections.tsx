import { CheckCircle2 } from "lucide-react";
import type { ArchiveManifest, OperationProgress } from "../../shared/types";
import {
  formatBytes,
  operationProgressLabel,
  operationProgressPercent
} from "../../shared/format";
import {
  archiveStatusLabel,
  archiveValidationTitle
} from "../../shared/archive-validation";
import { type MessageKey, type TranslateFn, useI18n } from "../../i18n";
import styles from "./status-bar.module.css";

export type StatusSummaryState = {
  selectedCount: number;
  entryFilterActive: boolean;
  visibleEntryCount: number;
  filterableEntryCount: number;
  pendingChangeCount: number;
  hasPendingChanges: boolean;
  redoAvailable: boolean;
  entryCount: number | null;
};

export type ArchiveStatusState = {
  manifest: ArchiveManifest | null;
  hasPendingChanges: boolean;
};

export function statusSummary({
  selectedCount,
  entryFilterActive,
  visibleEntryCount,
  filterableEntryCount,
  pendingChangeCount,
  hasPendingChanges,
  redoAvailable,
  entryCount
}: StatusSummaryState, t: TranslateFn) {
  if (selectedCount > 0) {
    return t("status.selected", { count: selectedCount });
  }
  if (entryFilterActive && entryCount !== null) {
    return t("status.shown", { visible: visibleEntryCount, total: filterableEntryCount });
  }
  if (hasPendingChanges) {
    return t("status.pending", { count: pendingChangeCount });
  }
  if (redoAvailable) {
    return t("status.redoAvailable");
  }
  return entryCount !== null ? t("status.entries", { count: entryCount }) : t("status.noArchive");
}

export function OperationStatus({ operation }: { operation: OperationProgress }) {
  const { t } = useI18n();
  const percent = operationProgressPercent(operation);
  return (
    <span className={styles.operationStatus}>
      <span className={styles.operationMessage}>
        {localizedOperationLabel(operation.label, t)}:{" "}
        {operation.cancelRequested
          ? t("status.cancelRequested")
          : localizedOperationMessage(operation.message, t)}
      </span>
      <span
        className={`${styles.operationProgress}${percent === null ? ` ${styles.indeterminate}` : ""}`}
        role="progressbar"
        aria-label={operation.label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent ?? undefined}
      >
        <span
          style={{
            width: `${percent ?? 35}%`
          }}
        />
      </span>
      <span className={styles.operationProgressLabel}>{operationProgressLabel(operation)}</span>
    </span>
  );
}

export function ArchiveMetrics({ manifest }: { manifest: ArchiveManifest | null }) {
  const { t } = useI18n();
  return (
    <>
      <span>{manifest ? manifest.formatKind.toUpperCase() : t("status.noFormat")}</span>
      <span>
        {manifest
          ? t("status.sizeValue", { size: formatBytes(manifest.totalUncompressedSize) })
          : t("status.sizeEmpty")}
      </span>
      <span>
        {manifest?.totalCompressedSize === null || !manifest
          ? t("status.packedEmpty")
          : t("status.packedValue", { size: formatBytes(manifest.totalCompressedSize) })}
      </span>
    </>
  );
}

export function ArchiveValidationStatus({
  archive: { manifest, hasPendingChanges }
}: {
  archive: ArchiveStatusState;
}) {
  const { t } = useI18n();
  if (
    !manifest ||
    (!hasPendingChanges &&
      !manifest.validation.fullyValidated &&
      !manifest.validation.passwordRequired)
  ) {
    return null;
  }
  return (
    <span title={archiveValidationTitle(manifest.validation, hasPendingChanges, t)}>
      {archiveStatusLabel(manifest.validation, hasPendingChanges, t)}
    </span>
  );
}

export function StatusMessage({ message }: { message: string }) {
  const { t } = useI18n();
  return (
    <span className={styles.statusOk}>
      <CheckCircle2 size={15} aria-hidden="true" />
      {localizedStatusMessage(message, t)}
    </span>
  );
}

function localizedStatusMessage(message: string, t: TranslateFn) {
  const exact = localizedExactStatusMessage(message, t);
  if (exact) {
    return exact;
  }
  const opened = message.match(/^Opened (.+)$/);
  if (opened) {
    return t("feedback.opened", { name: opened[1] });
  }
  const created = message.match(/^Created (.+)$/);
  if (created) {
    return t("feedback.created", { name: created[1] });
  }
  const saved = message.match(/^Saved (.+)$/);
  if (saved) {
    return t("feedback.saved", { name: saved[1] });
  }
  const testPassedSelected = message.match(/^Test passed: (\d+) selected$/);
  if (testPassedSelected) {
    return t("feedback.testPassedSelected", { count: testPassedSelected[1] });
  }
  const testPassed = message.match(/^Test passed: (.+)$/);
  if (testPassed) {
    return t("feedback.testPassed", { name: testPassed[1] });
  }
  const extractedSelected = message.match(/^Extracted (\d+) selected to (.+)$/);
  if (extractedSelected) {
    return t("feedback.extractedSelectedTo", {
      count: extractedSelected[1],
      path: extractedSelected[2]
    });
  }
  const extracted = message.match(/^Extracted to (.+)$/);
  if (extracted) {
    return t("feedback.extractedTo", { path: extracted[1] });
  }
  const pathsCopied = message.match(/^(\d+) paths copied$/);
  if (pathsCopied) {
    return t("feedback.pathsCopied", { count: pathsCopied[1] });
  }
  const failedWithCode = message.match(/^(Open|Test|Extract|Create|Add|Save) failed: (.+)$/);
  if (failedWithCode) {
    const key = failedStatusKey(failedWithCode[1]);
    if (key) {
      return t(key, { code: failedWithCode[2] });
    }
  }
  return message;
}

function localizedOperationLabel(label: string, t: TranslateFn) {
  const key = operationLabelKeys[label];
  return key ? t(key) : label;
}

function localizedOperationMessage(message: string, t: TranslateFn) {
  const key = operationMessageKeys[message];
  return key ? t(key) : message;
}

function localizedExactStatusMessage(message: string, t: TranslateFn) {
  const key = statusMessageKeys[message];
  return key ? t(key) : null;
}

function failedStatusKey(status: string): MessageKey | null {
  return failedStatusKeys[status] ?? null;
}

const operationLabelKeys: Record<string, MessageKey> = {
  "Open archive": "operation.openArchive",
  "Refresh archive": "operation.refreshArchive",
  "Test archive": "operation.testArchive",
  "Test selected entries": "operation.testSelectedEntries",
  "Extract archive": "operation.extractArchive",
  "Extract selected entries": "operation.extractSelectedEntries",
  "Create archive": "operation.createArchive",
  "Plan additions": "operation.planAdditions",
  "Save archive": "operation.saveArchive"
};

const operationMessageKeys: Record<string, MessageKey> = {
  "Operation queued": "operation.queued",
  "Operation running": "operation.running",
  "Operation finished": "operation.finished",
  "Operation failed": "operation.failed",
  "Operation canceled": "operation.canceled",
  "Cancel requested": "status.cancelRequested"
};

const statusMessageKeys: Record<string, MessageKey> = {
  Ready: "validation.ready",
  "Unsaved changes": "validation.unsavedChanges",
  "Password required": "validation.passwordRequired",
  "Cancel requested": "status.cancelRequested",
  "Opening archive": "feedback.openingArchive",
  "Choosing archive": "feedback.choosingArchive",
  "Open cancelled": "feedback.openCancelled",
  "Open dialog failed": "feedback.openDialogFailed",
  "Open failed": "feedback.openFailed",
  "Testing archive": "feedback.testingArchive",
  "Testing selected entries": "feedback.testingSelectedEntries",
  "Test failed": "feedback.testFailed",
  "Test passed; refresh failed": "feedback.testPassedRefreshFailed",
  "Extracting archive": "feedback.extractingArchive",
  "Extracting selected entries": "feedback.extractingSelectedEntries",
  "Extract failed": "feedback.extractFailed",
  "Extracted; refresh failed": "feedback.extractedRefreshFailed",
  "Creating archive": "feedback.creatingArchive",
  "Create failed": "feedback.createFailed",
  "Replace required": "feedback.replaceRequired",
  "Planning additions": "feedback.planningAdditions",
  "Add failed": "feedback.addFailed",
  "Saving archive": "feedback.savingArchive",
  "Save failed": "feedback.saveFailed",
  "Path copied": "feedback.pathCopied",
  "Copy failed": "feedback.copyFailed"
};

const failedStatusKeys: Record<string, MessageKey> = {
  Open: "feedback.openFailedWithCode",
  Test: "feedback.testFailedWithCode",
  Extract: "feedback.extractFailedWithCode",
  Create: "feedback.createFailedWithCode",
  Add: "feedback.addFailedWithCode",
  Save: "feedback.saveFailedWithCode"
};
