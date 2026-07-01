import { basename } from "../../shared/path-utils";
import type {
  CreateArchiveFeedback,
  CreateArchivePendingChanges,
  CreateArchivePrompts,
  CreateArchiveState,
  CreateArchiveTarget,
  RunCreateArchiveCommand
} from "./create-archive-action-types";

type CreateArchiveSubmitActionInput = {
  state: Pick<
    CreateArchiveState,
    | "canCreateArchive"
    | "createSingleStreamInputLimitExceeded"
    | "createOutputPath"
    | "createInputs"
    | "readCreatePassword"
    | "resetCreateForm"
  >;
  archive: CreateArchiveTarget;
  pendingChanges: Pick<CreateArchivePendingChanges, "resetAllPendingChanges">;
  feedback: CreateArchiveFeedback;
  prompts: Pick<CreateArchivePrompts, "setOverwritePrompt">;
  runCreateArchiveCommand: RunCreateArchiveCommand;
};

export function createCreateArchiveSubmitAction({
  state,
  archive: { setArchivePath, openArchivePath },
  pendingChanges: { resetAllPendingChanges },
  feedback: { setError, setStatus },
  prompts: { setOverwritePrompt },
  runCreateArchiveCommand
}: CreateArchiveSubmitActionInput) {
  async function createArchive(overwrite = false) {
    if (!state.canCreateArchive) {
      setError(
        state.createSingleStreamInputLimitExceeded
          ? "Single-stream outputs require exactly one file input"
          : "Choose archive output and at least one input"
      );
      setStatus("Create failed");
      return;
    }

    const passwordResult = state.readCreatePassword();
    if (passwordResult.kind === "rejected") {
      setError(passwordResult.message);
      setStatus("Create failed");
      return;
    }

    const result = await runCreateArchiveCommand({
      outputPath: state.createOutputPath,
      inputs: state.createInputs,
      password: passwordResult.password,
      overwrite
    });
    if (!result) {
      return;
    }
    setOverwritePrompt(null);
    state.resetCreateForm();
    resetAllPendingChanges();
    setArchivePath(result.archivePath);
    setStatus(`Created ${basename(result.archivePath)}`);
    void openArchivePath(result.archivePath);
  }

  return {
    createArchive
  };
}
