import type { ArchiveManifest } from "../../shared/types";
import type { FeedbackPort } from "../workflow-ports";
import type { createArchivePayloadActions } from "./archive-payload-actions";

type ArchivePayloadActions = ReturnType<typeof createArchivePayloadActions>;

type ArchivePayloadTestRequestsInput = {
  manifest: ArchiveManifest | null;
  selectedPaths: string[];
  feedback: Pick<FeedbackPort, "setError" | "setStatus">;
  actions: Pick<ArchivePayloadActions, "runArchiveTest" | "runSelectedEntriesTest">;
};

export function createArchivePayloadTestRequests({
  manifest,
  selectedPaths,
  feedback: { setError, setStatus },
  actions: { runArchiveTest, runSelectedEntriesTest }
}: ArchivePayloadTestRequestsInput) {
  async function runStartupTest(opened: ArchiveManifest) {
    await runArchiveTest(opened);
  }

  async function testArchive(password?: string) {
    if (!manifest) {
      setError("Open an archive before testing");
      setStatus("Test failed");
      return;
    }

    await runArchiveTest(manifest, password);
  }

  async function testSelectedEntries(password?: string) {
    if (!manifest || selectedPaths.length === 0) {
      setError("Select archive entries before testing");
      setStatus("Test failed");
      return;
    }

    await runSelectedEntriesTest({
      manifest,
      selectedPaths,
      password
    });
  }

  return {
    runStartupTest,
    testArchive,
    testSelectedEntries
  };
}
