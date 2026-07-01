import { parentDirectory } from "../../shared/path-utils";
import type { ArchiveManifest } from "../../shared/types";
import type { FeedbackPort } from "../workflow-ports";
import type { createArchivePayloadActions } from "./archive-payload-actions";

type ArchivePayloadActions = ReturnType<typeof createArchivePayloadActions>;

type ArchivePayloadExtractRequestsInput = {
  manifest: ArchiveManifest | null;
  selectedPaths: string[];
  destinationPathRef: {
    current: string;
  };
  updateDestinationPath: (path: string) => void;
  feedback: Pick<FeedbackPort, "setError" | "setStatus">;
  actions: Pick<
    ArchivePayloadActions,
    "runArchiveExtract" | "runSelectedEntriesExtract"
  >;
};

export function createArchivePayloadExtractRequests({
  manifest,
  selectedPaths,
  destinationPathRef,
  updateDestinationPath,
  feedback: { setError, setStatus },
  actions: { runArchiveExtract, runSelectedEntriesExtract }
}: ArchivePayloadExtractRequestsInput) {
  async function runStartupExtract(opened: ArchiveManifest) {
    await runArchiveExtract({
      target: opened,
      outputPath: "",
      password: undefined,
      overwrite: false
    });
  }

  async function extractArchive(password?: string, overwrite = false) {
    if (!manifest) {
      setError("Open an archive before extracting");
      setStatus("Extract failed");
      return;
    }

    await runArchiveExtract({
      target: manifest,
      outputPath: destinationPathRef.current,
      password,
      overwrite
    });
  }

  async function extractSelectedEntries(
    password?: string,
    overwrite = false,
    outputPathOverride?: string
  ) {
    if (!manifest || selectedPaths.length === 0) {
      setError("Select archive entries before extracting");
      setStatus("Extract failed");
      return;
    }

    await runSelectedEntriesExtract({
      manifest,
      selectedPaths,
      outputPath: outputPathOverride ?? destinationPathRef.current,
      password,
      overwrite
    });
  }

  async function extractSelectedEntriesHere() {
    if (!manifest) {
      setError("Open an archive before extracting");
      setStatus("Extract failed");
      return;
    }
    const outputPath = parentDirectory(manifest.archivePath);
    if (!outputPath) {
      setError("Archive folder could not be detected");
      setStatus("Extract failed");
      return;
    }
    updateDestinationPath(outputPath);
    await extractSelectedEntries(undefined, false, outputPath);
  }

  return {
    runStartupExtract,
    extractArchive,
    extractSelectedEntries,
    extractSelectedEntriesHere
  };
}
