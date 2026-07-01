export type ListEntry = {
  path: string;
  entryType: string;
  uncompressedSize: number;
  compressedSize: number | null;
  encrypted: boolean;
};

export type DirectEditStatus = {
  allowed: boolean;
  reason: string | null;
};

export type ArchiveValidation = {
  metadataValidated: boolean;
  payloadValidated: boolean;
  passwordRequired: boolean;
  fullyValidated: boolean;
  state: string;
  reason: string;
};

export type ArchiveManifest = {
  archivePath: string;
  archiveName: string;
  formatKind: string;
  formatSuffix: string;
  digestSha256: string;
  entries: ListEntry[];
  entryCount: number;
  totalUncompressedSize: number;
  totalCompressedSize: number | null;
  encryptedEntryCount: number;
  validation: ArchiveValidation;
  directEdit: DirectEditStatus;
};

export type CommandError = {
  code?: string;
  message?: string;
};

export type ExtractResult = {
  outputPath: string;
};

export type CreateResult = {
  archivePath: string;
};

export type StartupAction = "open" | "test" | "extract";

export type StartupRequest = {
  action: StartupAction;
  archivePath: string;
};

export type OperationPhase =
  | "started"
  | "running"
  | "scanning"
  | "reading"
  | "writing"
  | "testing"
  | "extracting"
  | "committing"
  | "cancelRequested"
  | "finished"
  | "failed"
  | "canceled";

export type OperationProgress = {
  id: number;
  label: string;
  phase: OperationPhase;
  message: string;
  cancelRequested: boolean;
  cancellable: boolean;
  processed: number | null;
  total: number | null;
};

export type OperationRunner = <T>(
  label: string,
  work: (operationId: number) => Promise<T>
) => Promise<T>;

export type DirectEditPlannedEntry = {
  archivePath: string;
  entryType: string;
};

export type DirectEditAddPlan = {
  additions: DirectEditPlannedEntry[];
  replacements: DirectEditPlannedEntry[];
};

export type PasswordAction = "test" | "extract" | "testSelection" | "extractSelection";
export type OverwriteAction = "extract" | "extractSelection" | "create";
export type DropState = "idle" | "hover";
export type EntrySortColumn =
  | "path"
  | "entryType"
  | "uncompressedSize"
  | "compressedSize"
  | "encrypted";
export type EntrySortDirection = "asc" | "desc";

export type EntrySortState = {
  column: EntrySortColumn;
  direction: EntrySortDirection;
};

export type OverwritePromptState = {
  action: OverwriteAction;
  message: string;
};

export type DirectEditReplacePromptState = {
  inputs: string[];
  plan: DirectEditAddPlan;
  acceptedReplacements: DirectEditPlannedEntry[];
  replacementCount: number;
};

export type UnsavedAction =
  | { kind: "openArchive"; path: string }
  | { kind: "startupRequest"; request: StartupRequest }
  | { kind: "newArchive" };

export type UnsavedPromptState = {
  action: UnsavedAction;
  message: string;
};

export type CloseBlockedPromptState = {
  message: string;
  activeLabels: string[];
};

export type PendingChangesSnapshot = {
  deletePaths: string[];
  addInputs: string[];
  addEntries: DirectEditPlannedEntry[];
  replaceEntries: string[];
};
