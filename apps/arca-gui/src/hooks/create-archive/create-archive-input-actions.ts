import {
  chooseCreateArchiveOutput,
  chooseCreateInputFiles,
  chooseCreateInputFolder
} from "../../api/file-dialogs";
import type {
  CreateArchiveFeedback,
  CreateArchiveState
} from "./create-archive-action-types";

type CreateArchiveInputActionsInput = {
  state: Pick<
    CreateArchiveState,
    | "createOutputPath"
    | "createSingleStreamOutput"
    | "setCreateOutputPath"
    | "appendCreateInputs"
  >;
  feedback: CreateArchiveFeedback;
};

export function createCreateArchiveInputActions({
  state,
  feedback: { setError, setStatus }
}: CreateArchiveInputActionsInput) {
  async function chooseCreateOutput() {
    setError(null);
    setStatus("Choosing archive output");
    try {
      const selected = await chooseCreateArchiveOutput(state.createOutputPath);
      if (!selected) {
        setStatus("Output cancelled");
        return;
      }
      state.setCreateOutputPath(selected);
      setStatus("Archive output selected");
    } catch (caught) {
      setError(String(caught));
      setStatus("Output dialog failed");
    }
  }

  async function chooseCreateInput(
    choosingStatus: string,
    failedStatus: string,
    choose: () => Promise<string | string[] | null>
  ) {
    setError(null);
    setStatus(choosingStatus);
    try {
      const selected = await choose();
      appendCreateInputs(selected);
    } catch (caught) {
      setError(String(caught));
      setStatus(failedStatus);
    }
  }

  async function addCreateFiles() {
    await chooseCreateInput("Choosing files", "File dialog failed", () =>
      chooseCreateInputFiles(!state.createSingleStreamOutput)
    );
  }

  async function addCreateFolder() {
    if (state.createSingleStreamOutput) {
      setError("Single-stream outputs require exactly one file input");
      setStatus("Add unavailable");
      return;
    }
    await chooseCreateInput("Choosing folder", "Folder dialog failed", chooseCreateInputFolder);
  }

  function appendCreateInputs(selected: string | string[] | null) {
    const result = state.appendCreateInputs(selected);
    if (result.kind === "cancelled") {
      setStatus("Input cancelled");
      return;
    }
    if (result.kind === "rejected") {
      setError(result.message);
      setStatus("Input rejected");
      return;
    }
    setStatus(`${result.count} input${result.count === 1 ? "" : "s"} added`);
  }

  return {
    chooseCreateOutput,
    addCreateFiles,
    addCreateFolder,
    appendCreateInputs
  };
}
