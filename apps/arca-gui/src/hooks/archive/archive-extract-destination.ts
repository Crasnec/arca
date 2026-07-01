import React from "react";
import { chooseExtractDirectory, chooseExtractOutput } from "../../api/file-dialogs";
import { isSingleStreamManifest } from "../../shared/archive-validation";
import type { ArchiveManifest } from "../../shared/types";
import type { FeedbackPort } from "../workflow-ports";

type ArchiveExtractDestinationInput = {
  archive: {
    destinationPath: string;
    manifest: ArchiveManifest | null;
    setDestinationPath: React.Dispatch<React.SetStateAction<string>>;
  };
  feedback: Pick<FeedbackPort, "setError" | "setStatus">;
};

export function useArchiveExtractDestination({
  archive: { destinationPath, manifest, setDestinationPath },
  feedback: { setError, setStatus }
}: ArchiveExtractDestinationInput) {
  const destinationPathRef = React.useRef(destinationPath);

  React.useEffect(() => {
    destinationPathRef.current = destinationPath;
  }, [destinationPath]);

  function updateDestinationPath(path: string) {
    destinationPathRef.current = path;
    setDestinationPath(path);
  }

  async function chooseExtractDestination() {
    setError(null);
    setStatus("Choosing extract destination");
    try {
      const selected = isSingleStreamManifest(manifest)
        ? await chooseExtractOutput(destinationPath)
        : await chooseExtractDirectory(destinationPath);

      if (!selected) {
        setStatus("Destination cancelled");
        return;
      }
      updateDestinationPath(selected);
      setStatus("Destination selected");
    } catch (caught) {
      setError(String(caught));
      setStatus("Destination dialog failed");
    }
  }

  return {
    destinationPathRef,
    updateDestinationPath,
    chooseExtractDestination
  };
}
