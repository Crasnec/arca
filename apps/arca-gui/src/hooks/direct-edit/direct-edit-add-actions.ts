import type React from "react";
import {
  chooseDirectEditInputFiles,
  chooseDirectEditInputFolder
} from "../../api/file-dialogs";
import { normalizeSelectedPaths } from "../../shared/path-utils";
import type {
  ArchiveManifest,
  DirectEditPlannedEntry,
  DirectEditReplacePromptState
} from "../../shared/types";
import type { FeedbackPort } from "../workflow-ports";

type RunPlanDirectEditAdd = (input: {
  archivePath: string;
  inputs: string[];
  pendingDeleteEntries: string[];
  pendingAddEntries: DirectEditPlannedEntry[];
}) => Promise<{
  additions: DirectEditPlannedEntry[];
  replacements: DirectEditPlannedEntry[];
} | null>;

type DirectEditAddActionsInput = {
  manifest: ArchiveManifest | null;
  capability: {
    canAddDirectEdit: boolean;
  };
  pending: {
    pendingDeletePaths: string[];
    pendingAddEntries: DirectEditPlannedEntry[];
  };
  setDirectEditReplacePrompt: React.Dispatch<
    React.SetStateAction<DirectEditReplacePromptState | null>
  >;
  appendPendingAddPlan: (
    inputs: string[],
    additions: DirectEditPlannedEntry[],
    replacements: DirectEditPlannedEntry[]
  ) => void;
  reportDirectEditUnavailable: (unavailableStatus: string) => void;
  runPlanDirectEditAdd: RunPlanDirectEditAdd;
  feedback: Pick<FeedbackPort, "setError" | "setStatus">;
};

export function createDirectEditAddActions({
  manifest,
  capability: { canAddDirectEdit },
  pending: { pendingDeletePaths, pendingAddEntries },
  setDirectEditReplacePrompt,
  appendPendingAddPlan,
  reportDirectEditUnavailable,
  runPlanDirectEditAdd,
  feedback: { setError, setStatus }
}: DirectEditAddActionsInput) {
  async function chooseDirectEditInput(
    choosingStatus: string,
    failedStatus: string,
    choose: () => Promise<string | string[] | null>
  ) {
    setError(null);
    if (!canAddDirectEdit) {
      reportDirectEditUnavailable("Add unavailable");
      return;
    }
    setStatus(choosingStatus);
    try {
      const selected = await choose();
      await planDirectEditAdd(selected);
    } catch (caught) {
      setError(String(caught));
      setStatus(failedStatus);
    }
  }

  async function chooseDirectEditFiles() {
    await chooseDirectEditInput(
      "Choosing files",
      "File dialog failed",
      chooseDirectEditInputFiles
    );
  }

  async function chooseDirectEditFolder() {
    await chooseDirectEditInput(
      "Choosing folder",
      "Folder dialog failed",
      chooseDirectEditInputFolder
    );
  }

  async function planDirectEditAdd(selected: string | string[] | null) {
    const inputs = normalizeSelectedPaths(selected);
    if (inputs.length === 0) {
      setStatus("Add cancelled");
      return;
    }
    if (!manifest?.directEdit.allowed) {
      reportDirectEditUnavailable("Add unavailable");
      return;
    }

    const plan = await runPlanDirectEditAdd({
      archivePath: manifest.archivePath,
      inputs,
      pendingDeleteEntries: pendingDeletePaths,
      pendingAddEntries
    });
    if (!plan) {
      return;
    }
    if (plan.replacements.length > 0) {
      setDirectEditReplacePrompt({
        inputs,
        plan,
        acceptedReplacements: [],
        replacementCount: plan.replacements.length
      });
      setStatus("Replacement required");
      return;
    }
    appendPendingAddPlan(inputs, plan.additions, []);
  }

  return {
    chooseDirectEditFiles,
    chooseDirectEditFolder,
    planDirectEditAdd
  };
}
