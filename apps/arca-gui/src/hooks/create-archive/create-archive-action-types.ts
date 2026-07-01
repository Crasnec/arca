import type React from "react";
import type { ArchiveManifest, CreateResult } from "../../shared/types";
import type {
  FeedbackPort,
  OverwritePromptPort,
  UnsavedPromptPort
} from "../workflow-ports";
import type { useCreateArchiveState } from "./create-archive-state";

export type CreateArchiveState = ReturnType<typeof useCreateArchiveState>;

export type RunCreateArchiveCommand = (input: {
  outputPath: string;
  inputs: string[];
  password?: string;
  overwrite: boolean;
}) => Promise<CreateResult | null>;

export type CreateArchiveTarget = {
  setArchivePath: React.Dispatch<React.SetStateAction<string>>;
  openArchivePath: (path: string) => Promise<ArchiveManifest | null>;
};

export type CreateArchivePendingChanges = {
  hasPendingChanges: boolean;
  pendingChangeCount: number;
  resetAllPendingChanges: () => void;
};

export type CreateArchiveFeedback = Pick<FeedbackPort, "setError" | "setStatus">;
export type CreateArchivePrompts = OverwritePromptPort & UnsavedPromptPort;
