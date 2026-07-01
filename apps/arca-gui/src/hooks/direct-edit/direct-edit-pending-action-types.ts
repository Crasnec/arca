import type React from "react";
import type {
  ArchiveManifest,
  DirectEditPlannedEntry,
  DirectEditReplacePromptState
} from "../../shared/types";
import type { usePendingChanges } from "./pending-changes";

export type RunSaveDirectEditInput = {
  archivePath: string;
  expectedDigestSha256: string;
  deleteEntries: string[];
  addInputs: string[];
  addEntries: string[];
  replaceEntries: string[];
};

export type DirectEditPendingFeedback = {
  setError: React.Dispatch<React.SetStateAction<string | null>>;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
};

export type DirectEditPendingState = {
  pendingDeletePaths: string[];
  pendingAddInputs: string[];
  pendingAddEntries: DirectEditPlannedEntry[];
  pendingReplaceEntries: string[];
  hasPendingChanges: boolean;
};

export type DirectEditPendingActionsContext = {
  manifest: ArchiveManifest | null;
  selectedPaths: string[];
  pendingChanges: ReturnType<typeof usePendingChanges>;
  pending: DirectEditPendingState;
  setDirectEditReplacePrompt: React.Dispatch<
    React.SetStateAction<DirectEditReplacePromptState | null>
  >;
  setManifest: React.Dispatch<React.SetStateAction<ArchiveManifest | null>>;
  resetSelection: () => void;
  reportDirectEditUnavailable: (unavailableStatus: string) => void;
  runSaveDirectEdit: (input: RunSaveDirectEditInput) => Promise<ArchiveManifest | null>;
} & DirectEditPendingFeedback;
