import type { WorkbenchLayoutProps } from "../../components/workbench";
import { buildModalsModel } from "../workbench-modal-model";
import { buildAddressBarModel } from "./address-bar-model";
import { buildCommandBarModel } from "./command-bar-model";
import { buildEntryWorkspaceModel } from "./entry-workspace-model";
import { buildStatusBarModel } from "./status-bar-model";
import type { WorkbenchLayoutModelInput } from "./types";

export function buildWorkbenchLayoutModel(
  input: WorkbenchLayoutModelInput
): WorkbenchLayoutProps {
  return {
    commandBar: buildCommandBarModel(input),
    addressBar: buildAddressBarModel(input),
    entryWorkspace: buildEntryWorkspaceModel(input),
    statusBar: buildStatusBarModel(input),
    modals: buildModalsModel({
      create: {
        state: input.create.state,
        actions: input.create.actions,
        dropState: input.ui.dropState,
        loading: input.feedback.loading
      },
      prompts: {
        overwritePrompt: input.prompts.overwritePrompt,
        unsavedPrompt: input.prompts.unsavedPrompt,
        closeBlockedPrompt: input.prompts.closeBlockedPrompt,
        passwordAction: input.prompts.passwordAction
      },
      directEditReplacement: input.directEdit.replacement,
      dialogs: input.dialogs,
      manifest: input.archiveState.manifest,
      selectedEntryInfo: {
        entries: input.archive.selection.selectedEntries,
        uncompressedSize: input.archive.selection.selectedUncompressedSize,
        compressedSize: input.archive.selection.selectedCompressedSize,
        encryptedCount: input.archive.selection.selectedEncryptedCount
      },
      passwordInputRef: input.refs.passwordInputRef
    })
  };
}
