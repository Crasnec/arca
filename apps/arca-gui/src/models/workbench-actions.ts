export type WorkbenchActionGroups = {
  archive: {
    chooseArchive: () => void | Promise<void>;
    extractArchive: () => void | Promise<void>;
    extractSelectedEntries: () => void | Promise<void>;
    extractSelectedEntriesHere: () => void | Promise<void>;
    testArchive: () => void | Promise<void>;
    testSelectedEntries: () => void | Promise<void>;
  };
  browser: {
    copySelectedPaths: () => void | Promise<void>;
    focusEntryFilter: () => void;
  };
  create: {
    openCreateModal: () => void;
    closeCreateModal: () => void;
  };
  directEdit: {
    chooseDirectEditFiles: () => void | Promise<void>;
    chooseDirectEditFolder: () => void | Promise<void>;
    markSelectedForDelete: () => void;
  };
  dialogs: {
    openArchiveInfo: () => void;
    openSelectedEntryInfo: () => void;
    closeDirectEditReplacePrompt: () => void;
    closeOverwritePrompt: () => void;
    closeUnsavedPrompt: () => void;
    closeCloseBlockedPrompt: () => void;
    closeArchiveInfo: () => void;
    closeSelectedEntryInfo: () => void;
    closePasswordPrompt: () => void;
  };
  operation: {
    cancelActiveOperation: () => void | Promise<void>;
  };
  pendingChanges: {
    savePendingChanges: () => void | Promise<void>;
    undoPendingChangeSet: () => void;
    redoPendingChangeSet: () => void;
  };
};

type ArchiveOpenActionSource = Pick<WorkbenchActionGroups["archive"], "chooseArchive">;
type ArchivePayloadActionSource = Omit<WorkbenchActionGroups["archive"], "chooseArchive">;
type DirectEditActionSource = WorkbenchActionGroups["directEdit"] &
  WorkbenchActionGroups["pendingChanges"];
type DirectEditReplacementActionSource = Pick<
  WorkbenchActionGroups["dialogs"],
  "closeDirectEditReplacePrompt"
>;
type DialogActionSource = {
  archiveInfo: {
    openDialog: WorkbenchActionGroups["dialogs"]["openArchiveInfo"];
    close: WorkbenchActionGroups["dialogs"]["closeArchiveInfo"];
  };
  entryInfo: {
    openDialog: WorkbenchActionGroups["dialogs"]["openSelectedEntryInfo"];
    close: WorkbenchActionGroups["dialogs"]["closeSelectedEntryInfo"];
  };
  overwrite: {
    close: WorkbenchActionGroups["dialogs"]["closeOverwritePrompt"];
  };
  unsaved: {
    close: WorkbenchActionGroups["dialogs"]["closeUnsavedPrompt"];
  };
  closeBlocked: {
    close: WorkbenchActionGroups["dialogs"]["closeCloseBlockedPrompt"];
  };
  password: {
    close: WorkbenchActionGroups["dialogs"]["closePasswordPrompt"];
  };
};

export type WorkbenchActionGroupInput = {
  archiveOpen: ArchiveOpenActionSource;
  archivePayload: ArchivePayloadActionSource;
  browser: WorkbenchActionGroups["browser"];
  create: WorkbenchActionGroups["create"];
  directEdit: DirectEditActionSource;
  directEditReplacement: DirectEditReplacementActionSource;
  dialogs: DialogActionSource;
  operation: WorkbenchActionGroups["operation"];
};

export function buildWorkbenchActionGroups({
  archiveOpen,
  archivePayload,
  browser,
  create,
  directEdit,
  directEditReplacement,
  dialogs,
  operation
}: WorkbenchActionGroupInput): WorkbenchActionGroups {
  return {
    archive: {
      chooseArchive: archiveOpen.chooseArchive,
      extractArchive: archivePayload.extractArchive,
      extractSelectedEntries: archivePayload.extractSelectedEntries,
      extractSelectedEntriesHere: archivePayload.extractSelectedEntriesHere,
      testArchive: archivePayload.testArchive,
      testSelectedEntries: archivePayload.testSelectedEntries
    },
    browser,
    create,
    directEdit: {
      chooseDirectEditFiles: directEdit.chooseDirectEditFiles,
      chooseDirectEditFolder: directEdit.chooseDirectEditFolder,
      markSelectedForDelete: directEdit.markSelectedForDelete
    },
    dialogs: {
      openArchiveInfo: dialogs.archiveInfo.openDialog,
      openSelectedEntryInfo: dialogs.entryInfo.openDialog,
      closeDirectEditReplacePrompt:
        directEditReplacement.closeDirectEditReplacePrompt,
      closeOverwritePrompt: dialogs.overwrite.close,
      closeUnsavedPrompt: dialogs.unsaved.close,
      closeCloseBlockedPrompt: dialogs.closeBlocked.close,
      closeArchiveInfo: dialogs.archiveInfo.close,
      closeSelectedEntryInfo: dialogs.entryInfo.close,
      closePasswordPrompt: dialogs.password.close
    },
    operation,
    pendingChanges: {
      savePendingChanges: directEdit.savePendingChanges,
      undoPendingChangeSet: directEdit.undoPendingChangeSet,
      redoPendingChangeSet: directEdit.redoPendingChangeSet
    }
  };
}
