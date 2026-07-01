import {
  planDirectEditAddCommand,
  saveDirectEditCommand
} from "../../api/archive-commands";
import type {
  ArchiveManifest,
  CommandError,
  DirectEditAddPlan,
  DirectEditPlannedEntry,
} from "../../shared/types";
import type { FeedbackPort, OperationPort } from "../workflow-ports";

type DirectEditRunnerInput = {
  operation: OperationPort;
  feedback: FeedbackPort;
};

type RunPlanDirectEditAddInput = {
  archivePath: string;
  inputs: string[];
  pendingDeleteEntries: string[];
  pendingAddEntries: DirectEditPlannedEntry[];
};

type RunSaveDirectEditInput = {
  archivePath: string;
  expectedDigestSha256: string;
  deleteEntries: string[];
  addInputs: string[];
  addEntries: string[];
  replaceEntries: string[];
};

export function createDirectEditRunner({
  operation: { withOperation },
  feedback: { setLoading, setError, setStatus }
}: DirectEditRunnerInput) {
  function reportCommandFailure(caught: unknown, failedStatus: string) {
    const commandError = caught as CommandError;
    setError(commandError.message ?? String(caught));
    setStatus(commandError.code ? `${failedStatus}: ${commandError.code}` : failedStatus);
  }

  async function runPlanDirectEditAdd({
    archivePath,
    inputs,
    pendingDeleteEntries,
    pendingAddEntries
  }: RunPlanDirectEditAddInput): Promise<DirectEditAddPlan | null> {
    setLoading(true);
    setError(null);
    setStatus("Planning additions");
    try {
      return await withOperation("Plan additions", (operationId) =>
        planDirectEditAddCommand({
          archivePath,
          inputs,
          pendingDeleteEntries,
          pendingAddEntries,
          operationId
        })
      );
    } catch (caught) {
      reportCommandFailure(caught, "Add failed");
      return null;
    } finally {
      setLoading(false);
    }
  }

  async function runSaveDirectEdit({
    archivePath,
    expectedDigestSha256,
    deleteEntries,
    addInputs,
    addEntries,
    replaceEntries
  }: RunSaveDirectEditInput): Promise<ArchiveManifest | null> {
    setLoading(true);
    setError(null);
    setStatus("Saving archive");
    try {
      return await withOperation("Save archive", (operationId) =>
        saveDirectEditCommand({
          archivePath,
          expectedDigestSha256,
          deleteEntries,
          addInputs,
          addEntries,
          replaceEntries,
          operationId
        })
      );
    } catch (caught) {
      reportCommandFailure(caught, "Save failed");
      return null;
    } finally {
      setLoading(false);
    }
  }

  return {
    runPlanDirectEditAdd,
    runSaveDirectEdit
  };
}
