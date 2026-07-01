import {
  ArchiveInfoModal,
  type ArchiveInfoModalProps,
  EntryInfoModal,
  type EntryInfoModalProps
} from "./info-modals";
import {
  CloseBlockedModal,
  type CloseBlockedModalProps,
  DirectEditReplaceModal,
  type DirectEditReplaceModalProps,
  OverwritePromptModal,
  type OverwritePromptModalProps,
  PasswordPromptModal,
  type PasswordPromptModalProps,
  UnsavedChangesModal,
  type UnsavedChangesModalProps
} from "./prompt-modals";
import {
  CreateArchiveModal,
  type CreateArchiveModalProps
} from "./create-archive-modal";

export type ArchiveModalsProps = {
  create: CreateArchiveModalProps;
  overwrite: OverwritePromptModalProps;
  directEditReplace: DirectEditReplaceModalProps;
  unsaved: UnsavedChangesModalProps;
  closeBlocked: CloseBlockedModalProps;
  archiveInfo: ArchiveInfoModalProps;
  entryInfo: EntryInfoModalProps;
  password: PasswordPromptModalProps;
};

export function ArchiveModals({
  create,
  overwrite,
  directEditReplace,
  unsaved,
  closeBlocked,
  archiveInfo,
  entryInfo,
  password
}: ArchiveModalsProps) {
  return (
    <>
      <CreateArchiveModal {...create} />
      <OverwritePromptModal {...overwrite} />
      <DirectEditReplaceModal {...directEditReplace} />
      <UnsavedChangesModal {...unsaved} />
      <CloseBlockedModal {...closeBlocked} />
      <ArchiveInfoModal {...archiveInfo} />
      <EntryInfoModal {...entryInfo} />
      <PasswordPromptModal {...password} />
    </>
  );
}
