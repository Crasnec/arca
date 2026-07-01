import type React from "react";
import type { WorkbenchLayoutProps } from "../../components/workbench";
import type { WorkbenchActionGroups } from "../workbench-actions";
import type { ModalsModelInput } from "../workbench-modal-model";
import type {
  ArchiveManifest,
  DirectEditPlannedEntry,
  DropState,
  EntrySortState,
  ListEntry
} from "../../shared/types";

type WorkbenchModals = WorkbenchLayoutProps["modals"];

type ArchiveStateModelSource = {
  archivePath: string;
  manifest: ArchiveManifest | null;
  setArchivePath: React.Dispatch<React.SetStateAction<string>>;
};

type FeedbackModelSource = {
  loading: boolean;
  status: string;
};

type PromptModelSource = {
  passwordAction: WorkbenchModals["password"]["action"];
  overwritePrompt: WorkbenchModals["overwrite"]["prompt"];
  unsavedPrompt: WorkbenchModals["unsaved"]["prompt"];
  closeBlockedPrompt: WorkbenchModals["closeBlocked"]["prompt"];
};

type RefModelSource = {
  passwordInputRef: WorkbenchModals["password"]["inputRef"];
};

type OperationModelSource = {
  activeOperation: WorkbenchLayoutProps["statusBar"]["operation"];
};

type BrowserModelSource = {
  entryFilter: string;
  entrySort: EntrySortState;
  entryFilterRef: React.RefObject<HTMLInputElement | null>;
  selectEntry: (
    path: string,
    event: React.MouseEvent<HTMLTableRowElement>,
    visibleEntries: ListEntry[]
  ) => void;
  updateEntryFilter: (value: string) => void;
  clearEntryFilter: () => void;
  changeEntrySort: WorkbenchLayoutProps["entryWorkspace"]["actions"]["onSort"];
  openEntryContextMenu: WorkbenchLayoutProps["entryWorkspace"]["actions"]["onOpenEntryContextMenu"];
};

type ArchiveModelSource = {
  open: {
    openArchive: WorkbenchLayoutProps["addressBar"]["actions"]["onSubmit"];
    chooseArchive: WorkbenchLayoutProps["addressBar"]["actions"]["onChooseArchive"];
  };
  table: {
    treeRows: string[];
    entryFilterActive: boolean;
    visibleEntryCount: number;
    filterableEntryCount: number;
    visibleEntries: ListEntry[];
    visiblePendingAddEntries: DirectEditPlannedEntry[];
    selectedPathSet: Set<string>;
  };
  selection: {
    selectedEntries: WorkbenchModals["entryInfo"]["entries"];
    selectedUncompressedSize: WorkbenchModals["entryInfo"]["uncompressedSize"];
    selectedCompressedSize: WorkbenchModals["entryInfo"]["compressedSize"];
    selectedEncryptedCount: WorkbenchModals["entryInfo"]["encryptedCount"];
  };
};

type CreateModelSource = {
  state: ModalsModelInput["create"]["state"];
  actions: ModalsModelInput["create"]["actions"];
};

type DirectEditModelSource = {
  pending: {
    pendingReplaceEntries: string[];
    pendingChangeCount: number;
    hasPendingChanges: boolean;
  };
  replacement: ModalsModelInput["directEditReplacement"];
};

type UiModelSource = {
  dropState: DropState;
  selectedCount: number;
  canAddDirectEdit: boolean;
};

type CapabilityModelSource = {
  hasArchive: boolean;
  redoAvailable: boolean;
  canSaveDirectEdit: boolean;
  canDeleteSelected: boolean;
  canUndoPendingChanges: boolean;
  canRedoPendingChanges: boolean;
};

export type WorkbenchLayoutModelInput = {
  archiveState: ArchiveStateModelSource;
  feedback: FeedbackModelSource;
  prompts: PromptModelSource;
  refs: RefModelSource;
  operation: OperationModelSource;
  browser: BrowserModelSource;
  archive: ArchiveModelSource;
  create: CreateModelSource;
  directEdit: DirectEditModelSource;
  ui: UiModelSource;
  capabilities: CapabilityModelSource;
  actions: WorkbenchActionGroups;
  dialogs: ModalsModelInput["dialogs"];
};

export type CommandBarModelInput = Pick<
  WorkbenchLayoutModelInput,
  "feedback" | "operation" | "directEdit" | "ui" | "capabilities" | "actions"
>;

export type AddressBarModelInput = Pick<
  WorkbenchLayoutModelInput,
  "archiveState" | "feedback" | "browser" | "archive"
>;

export type EntryWorkspaceModelInput = Pick<
  WorkbenchLayoutModelInput,
  "archiveState" | "browser" | "archive" | "directEdit" | "ui"
>;

export type StatusBarModelInput = Pick<
  WorkbenchLayoutModelInput,
  "archiveState" | "feedback" | "operation" | "archive" | "directEdit" | "ui" | "capabilities"
>;
