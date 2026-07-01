import type { ArchiveManifest, PasswordAction } from "../../shared/types";
import type {
  FeedbackPort,
  OperationPort,
  OverwritePromptPort
} from "../workflow-ports";

export type PayloadOperationRunner = (input: {
  startStatus: string;
  action: PasswordAction;
  failedStatus: string;
  run: () => Promise<void>;
}) => Promise<void>;

export type ArchivePayloadActionsInput = {
  operation: OperationPort;
  feedback: Pick<FeedbackPort, "setStatus">;
  prompts: OverwritePromptPort;
  refreshArchiveAfterValidation: (path: string, reason: string) => Promise<boolean>;
  runPayloadOperation: PayloadOperationRunner;
};

export type ArchiveExtractInput = {
  target: ArchiveManifest;
  outputPath: string;
  password?: string;
  overwrite: boolean;
};

export type SelectedEntriesInput = {
  manifest: ArchiveManifest;
  selectedPaths: string[];
  password?: string;
};

export type SelectedExtractInput = SelectedEntriesInput & {
  outputPath: string;
  overwrite: boolean;
};
