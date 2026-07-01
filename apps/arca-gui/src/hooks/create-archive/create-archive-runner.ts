import { createArchiveCommand } from "../../api/archive-commands";
import type { CommandError } from "../../shared/types";
import { isOverwritePromptError } from "../../shared/command-errors";
import type {
  FeedbackPort,
  OperationPort,
  OverwritePromptPort
} from "../workflow-ports";

type CreateArchiveRunnerInput = {
  operation: OperationPort;
  feedback: FeedbackPort;
  prompts: OverwritePromptPort;
};

type RunCreateArchiveInput = {
  outputPath: string;
  inputs: string[];
  password?: string;
  overwrite: boolean;
};

export function createArchiveRunner({
  operation: { withOperation },
  feedback: { setLoading, setError, setStatus },
  prompts: { setOverwritePrompt }
}: CreateArchiveRunnerInput) {
  async function runCreateArchiveCommand({
    outputPath,
    inputs,
    password,
    overwrite
  }: RunCreateArchiveInput) {
    setLoading(true);
    setError(null);
    setStatus("Creating archive");
    try {
      return await withOperation("Create archive", (operationId) =>
        createArchiveCommand({
          outputPath,
          inputs,
          password,
          overwrite,
          operationId
        })
      );
    } catch (caught) {
      const commandError = caught as CommandError;
      setError(commandError.message ?? String(caught));
      if (isOverwritePromptError(commandError)) {
        setOverwritePrompt({ action: "create", message: commandError.message ?? "" });
        setStatus("Replace required");
        return null;
      }
      setStatus(commandError.code ? `Create failed: ${commandError.code}` : "Create failed");
      return null;
    } finally {
      setLoading(false);
    }
  }

  return {
    runCreateArchiveCommand
  };
}
