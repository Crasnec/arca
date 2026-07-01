import {
  extractArchivePayloads,
  testArchivePayloads
} from "../../api/archive-commands";
import type { ArchiveManifest } from "../../shared/types";
import type {
  ArchiveExtractInput,
  ArchivePayloadActionsInput
} from "./archive-payload-action-types";

export function createArchivePayloadArchiveActions({
  operation: { withOperation },
  feedback: { setStatus },
  prompts: { setOverwritePrompt },
  refreshArchiveAfterValidation,
  runPayloadOperation
}: ArchivePayloadActionsInput) {
  async function runArchiveTest(target: ArchiveManifest, password?: string) {
    await runPayloadOperation({
      startStatus: "Testing archive",
      action: "test",
      failedStatus: "Test failed",
      run: async () => {
        await withOperation("Test archive", (operationId) =>
          testArchivePayloads({
            archivePath: target.archivePath,
            password,
            operationId
          })
        );
        const refreshed = await refreshArchiveAfterValidation(
          target.archivePath,
          "Test validated all archive payloads."
        );
        setStatus(refreshed ? `Test passed: ${target.archiveName}` : "Test passed; refresh failed");
      }
    });
  }

  async function runArchiveExtract({
    target,
    outputPath,
    password,
    overwrite
  }: ArchiveExtractInput) {
    await runPayloadOperation({
      startStatus: "Extracting archive",
      action: "extract",
      failedStatus: "Extract failed",
      run: async () => {
        const result = await withOperation("Extract archive", (operationId) =>
          extractArchivePayloads({
            archivePath: target.archivePath,
            outputPath,
            password,
            overwrite,
            operationId
          })
        );
        setOverwritePrompt(null);
        const refreshed = await refreshArchiveAfterValidation(
          target.archivePath,
          "Extract validated all archive payloads."
        );
        setStatus(refreshed ? `Extracted to ${result.outputPath}` : "Extracted; refresh failed");
      }
    });
  }

  return {
    runArchiveTest,
    runArchiveExtract
  };
}
