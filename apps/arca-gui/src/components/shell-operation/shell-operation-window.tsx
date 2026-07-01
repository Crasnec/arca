import React from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  extractArchivePayloads,
  testArchivePayloads
} from "../../api/archive-commands";
import { useI18n } from "../../i18n";
import { OPERATION_PROGRESS_EVENT } from "../../shared/constants";
import {
  formatBytes,
  operationProgressLabel,
  operationProgressPercent
} from "../../shared/format";
import { basename } from "../../shared/path-utils";
import type { CommandError, OperationProgress, StartupRequest } from "../../shared/types";
import styles from "./shell-operation.module.css";

type ShellRunState =
  | { kind: "queued" }
  | { kind: "running"; operationId: number }
  | { kind: "succeeded"; outputPath?: string }
  | { kind: "failed"; message: string };

export function ShellOperationWindow({ request }: { request: StartupRequest }) {
  const { t } = useI18n();
  const startedRef = React.useRef(false);
  const [state, setState] = React.useState<ShellRunState>({ kind: "queued" });
  const [operation, setOperation] = React.useState<OperationProgress | null>(null);
  const archiveName = basename(request.archivePath);
  const percent = operation ? operationProgressPercent(operation) : null;
  const progressLabel = operation ? operationProgressLabel(operation) : t("operation.queued");
  const title =
    request.action === "extract" ? t("shell.title.extract") : t("shell.title.test");
  const statusText = shellStatusText(request, state, operation, t);
  const isRunning = state.kind === "queued" || state.kind === "running";

  React.useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | null = null;

    listen<OperationProgress>(OPERATION_PROGRESS_EVENT, (event) => {
      const next = event.payload;
      if (!mounted) {
        return;
      }
      setOperation((current) => {
        if (current && current.id !== next.id) {
          return current;
        }
        return next;
      });
    })
      .then((value) => {
        unlisten = value;
      })
      .catch(() => undefined);

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  React.useEffect(() => {
    if (startedRef.current) {
      return;
    }
    startedRef.current = true;

    async function runShellOperation() {
      const label = shellOperationLabel(request);
      const operationId = await invoke<number>("begin_operation", { label });
      setState({ kind: "running", operationId });
      setOperation({
        id: operationId,
        label,
        phase: "started",
        message: "Operation queued",
        cancelRequested: false,
        cancellable: true,
        processed: null,
        total: null
      });

      try {
        if (request.action === "extract") {
          const result = await extractArchivePayloads({
            archivePath: request.archivePath,
            outputPath: "",
            overwrite: false,
            operationId
          });
          setState({ kind: "succeeded", outputPath: result.outputPath });
        } else {
          await testArchivePayloads({
            archivePath: request.archivePath,
            operationId
          });
          setState({ kind: "succeeded" });
        }
      } catch (caught) {
        const error = caught as CommandError;
        setState({ kind: "failed", message: error.message ?? String(caught) });
      } finally {
        await invoke("discard_operation", { operationId }).catch(() => undefined);
      }
    }

    void runShellOperation();

    return undefined;
  }, [request]);

  async function cancelOperation() {
    if (state.kind !== "running") {
      return;
    }
    setOperation((current) =>
      current
        ? {
            ...current,
            phase: "cancelRequested",
            message: "Cancel requested",
            cancelRequested: true,
            cancellable: false
          }
        : current
    );
    await invoke("cancel_operation", { operationId: state.operationId }).catch(() => undefined);
  }

  async function closeWindow() {
    await invoke("close_current_window").catch(() => undefined);
  }

  return (
    <main className={styles.shellOperation} aria-label={title}>
      <section className={styles.panel}>
        <div className={styles.header}>
          <div>
            <div className={styles.title}>{title}</div>
            <div className={styles.archiveName}>{archiveName}</div>
          </div>
          <span className={statusClassName(state.kind)}>
            {state.kind === "failed"
              ? t("operation.failed")
              : state.kind === "succeeded"
                ? t("operation.finished")
                : progressLabel}
          </span>
        </div>

        <div
          className={`${styles.progressTrack}${percent === null && isRunning ? ` ${styles.indeterminate}` : ""}`}
          role="progressbar"
          aria-label={title}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={percent ?? undefined}
        >
          <span style={{ width: `${percent ?? 32}%` }} />
        </div>

        <div className={styles.detailRow}>
          <span className={styles.statusText}>{statusText}</span>
          {operation?.processed !== null && operation?.processed !== undefined ? (
            <span className={styles.bytes}>
              {operation.total ? `${operation.processed}/${operation.total}` : formatBytes(operation.processed)}
            </span>
          ) : null}
        </div>

        <div className={styles.path} title={request.archivePath}>
          {request.archivePath}
        </div>

        <div className={styles.actions}>
          {isRunning ? (
            <button type="button" onClick={() => void cancelOperation()} disabled={operation?.cancelRequested}>
              {operation?.cancelRequested ? t("command.canceling") : t("command.cancel")}
            </button>
          ) : (
            <button type="button" onClick={() => void closeWindow()}>
              {t("modal.ok")}
            </button>
          )}
        </div>
      </section>
    </main>
  );
}

function shellOperationLabel(request: StartupRequest) {
  return request.action === "extract" ? "Extract archive" : "Test archive";
}

function shellStatusText(
  request: StartupRequest,
  state: ShellRunState,
  operation: OperationProgress | null,
  t: ReturnType<typeof useI18n>["t"]
) {
  if (state.kind === "failed") {
    return t("shell.failed", { message: state.message });
  }
  if (state.kind === "succeeded") {
    return request.action === "extract"
      ? t("shell.extractedTo", { path: state.outputPath ?? "" })
      : t("shell.testPassed");
  }
  if (operation?.cancelRequested) {
    return t("status.cancelRequested");
  }
  return request.action === "extract" ? t("shell.extracting") : t("shell.testing");
}

function statusClassName(kind: ShellRunState["kind"]) {
  if (kind === "failed") {
    return `${styles.badge} ${styles.badgeError}`;
  }
  if (kind === "succeeded") {
    return `${styles.badge} ${styles.badgeDone}`;
  }
  return styles.badge;
}
