import type { WorkbenchLayoutProps } from "../components/workbench";

type WorkbenchModals = WorkbenchLayoutProps["modals"];

export type CreateModalModelInput = {
  state: {
    createOpen: WorkbenchModals["create"]["open"];
    createOutputPath: WorkbenchModals["create"]["outputPath"];
    createSingleStreamOutput: WorkbenchModals["create"]["singleStreamOutput"];
    createInputs: WorkbenchModals["create"]["inputs"];
    createEncryptionAllowed: WorkbenchModals["create"]["encryptionAllowed"];
    createEncrypt: WorkbenchModals["create"]["encrypt"];
    canCreateArchive: WorkbenchModals["create"]["canCreate"];
    createPasswordInputRef: WorkbenchModals["create"]["passwordInputRef"];
  };
  actions: {
    setCreateOutputPath: WorkbenchModals["create"]["setOutputPath"];
    setCreateEncrypt: WorkbenchModals["create"]["setEncrypt"];
    chooseCreateOutput: WorkbenchModals["create"]["chooseOutput"];
    addCreateFiles: WorkbenchModals["create"]["addFiles"];
    addCreateFolder: WorkbenchModals["create"]["addFolder"];
    removeCreateInput: WorkbenchModals["create"]["removeInput"];
    closeCreateModal: WorkbenchModals["create"]["close"];
    createArchive: WorkbenchModals["create"]["create"];
  };
  dropState: WorkbenchModals["create"]["dropState"];
  loading: WorkbenchModals["create"]["loading"];
};

export type ModalsModelInput = {
  create: CreateModalModelInput;
  prompts: {
    overwritePrompt: WorkbenchModals["overwrite"]["prompt"];
    unsavedPrompt: WorkbenchModals["unsaved"]["prompt"];
    closeBlockedPrompt: WorkbenchModals["closeBlocked"]["prompt"];
    passwordAction: WorkbenchModals["password"]["action"];
  };
  directEditReplacement: {
    directEditReplacePrompt: WorkbenchModals["directEditReplace"]["prompt"];
    closeDirectEditReplacePrompt: WorkbenchModals["directEditReplace"]["close"];
    skipReplacementAddition: WorkbenchModals["directEditReplace"]["skip"];
    confirmReplacementAddition: WorkbenchModals["directEditReplace"]["replace"];
    skipReplacementAdditions: WorkbenchModals["directEditReplace"]["skipAll"];
    confirmReplacementAdditions: WorkbenchModals["directEditReplace"]["replaceAll"];
  };
  dialogs: {
    archiveInfo: Pick<WorkbenchModals["archiveInfo"], "open" | "close">;
    entryInfo: Pick<WorkbenchModals["entryInfo"], "open" | "close">;
    overwrite: Pick<WorkbenchModals["overwrite"], "close" | "confirm">;
    unsaved: Pick<WorkbenchModals["unsaved"], "close" | "discard">;
    closeBlocked: Pick<WorkbenchModals["closeBlocked"], "close">;
    password: Pick<WorkbenchModals["password"], "close" | "submit">;
  };
  manifest: WorkbenchModals["archiveInfo"]["manifest"];
  selectedEntryInfo: {
    entries: WorkbenchModals["entryInfo"]["entries"];
    uncompressedSize: WorkbenchModals["entryInfo"]["uncompressedSize"];
    compressedSize: WorkbenchModals["entryInfo"]["compressedSize"];
    encryptedCount: WorkbenchModals["entryInfo"]["encryptedCount"];
  };
  passwordInputRef: WorkbenchModals["password"]["inputRef"];
};

export function buildModalsModel({
  create,
  prompts,
  directEditReplacement,
  dialogs,
  manifest,
  selectedEntryInfo,
  passwordInputRef
}: ModalsModelInput): WorkbenchLayoutProps["modals"] {
  const { state: createState, actions: createActions, dropState, loading } = create;
  return {
    create: {
      open: createState.createOpen,
      dropState,
      outputPath: createState.createOutputPath,
      loading,
      singleStreamOutput: createState.createSingleStreamOutput,
      inputs: createState.createInputs,
      encryptionAllowed: createState.createEncryptionAllowed,
      encrypt: createState.createEncrypt,
      canCreate: createState.canCreateArchive,
      passwordInputRef: createState.createPasswordInputRef,
      setOutputPath: createActions.setCreateOutputPath,
      setEncrypt: createActions.setCreateEncrypt,
      chooseOutput: createActions.chooseCreateOutput,
      addFiles: createActions.addCreateFiles,
      addFolder: createActions.addCreateFolder,
      removeInput: createActions.removeCreateInput,
      close: createActions.closeCreateModal,
      create: createActions.createArchive
    },
    overwrite: {
      prompt: prompts.overwritePrompt,
      close: dialogs.overwrite.close,
      confirm: dialogs.overwrite.confirm
    },
    directEditReplace: {
      prompt: directEditReplacement.directEditReplacePrompt,
      close: directEditReplacement.closeDirectEditReplacePrompt,
      skip: directEditReplacement.skipReplacementAddition,
      replace: directEditReplacement.confirmReplacementAddition,
      skipAll: directEditReplacement.skipReplacementAdditions,
      replaceAll: directEditReplacement.confirmReplacementAdditions
    },
    unsaved: {
      prompt: prompts.unsavedPrompt,
      close: dialogs.unsaved.close,
      discard: dialogs.unsaved.discard
    },
    closeBlocked: {
      prompt: prompts.closeBlockedPrompt,
      close: dialogs.closeBlocked.close
    },
    archiveInfo: {
      open: dialogs.archiveInfo.open,
      manifest,
      close: dialogs.archiveInfo.close
    },
    entryInfo: {
      open: dialogs.entryInfo.open,
      entries: selectedEntryInfo.entries,
      uncompressedSize: selectedEntryInfo.uncompressedSize,
      compressedSize: selectedEntryInfo.compressedSize,
      encryptedCount: selectedEntryInfo.encryptedCount,
      close: dialogs.entryInfo.close
    },
    password: {
      action: prompts.passwordAction,
      archiveName: manifest?.archiveName ?? "Archive",
      inputRef: passwordInputRef,
      close: dialogs.password.close,
      submit: dialogs.password.submit
    }
  };
}
