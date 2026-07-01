import {
  extractSelectedArchiveEntries,
  testSelectedArchiveEntries
} from "../../api/archive-commands";
import type {
  ArchivePayloadActionsInput,
  SelectedEntriesInput,
  SelectedExtractInput
} from "./archive-payload-action-types";

type ArchivePayloadSelectionActionsInput = Pick<
  ArchivePayloadActionsInput,
  "operation" | "feedback" | "prompts" | "runPayloadOperation"
>;

export function createArchivePayloadSelectionActions({
  operation: { withOperation },
  feedback: { setStatus },
  prompts: { setOverwritePrompt },
  runPayloadOperation
}: ArchivePayloadSelectionActionsInput) {
  async function runSelectedEntriesTest({
    manifest,
    selectedPaths,
    password
  }: SelectedEntriesInput) {
    await runPayloadOperation({
      startStatus: "Testing selected entries",
      action: "testSelection",
      failedStatus: "Test failed",
      run: async () => {
        await withOperation("Test selected entries", (operationId) =>
          testSelectedArchiveEntries({
            archivePath: manifest.archivePath,
            entries: selectedPaths,
            password,
            operationId
          })
        );
        setStatus(`Test passed: ${selectedPaths.length} selected`);
      }
    });
  }

  async function runSelectedEntriesExtract({
    manifest,
    selectedPaths,
    outputPath,
    password,
    overwrite
  }: SelectedExtractInput) {
    await runPayloadOperation({
      startStatus: "Extracting selected entries",
      action: "extractSelection",
      failedStatus: "Extract failed",
      run: async () => {
        const result = await withOperation("Extract selected entries", (operationId) =>
          extractSelectedArchiveEntries({
            archivePath: manifest.archivePath,
            outputPath,
            entries: selectedPaths,
            password,
            overwrite,
            operationId
          })
        );
        setOverwritePrompt(null);
        setStatus(`Extracted ${selectedPaths.length} selected to ${result.outputPath}`);
      }
    });
  }

  return {
    runSelectedEntriesTest,
    runSelectedEntriesExtract
  };
}
