import {
  type WorkbenchInfoDialogsInput,
  useWorkbenchInfoDialogs
} from "./workbench-info-dialogs";
import {
  type WorkbenchPromptDialogsInput,
  useWorkbenchPromptDialogs
} from "./workbench-prompt-dialogs";

export type WorkbenchDialogsInput = WorkbenchPromptDialogsInput & {
  info: WorkbenchInfoDialogsInput;
};

export function useWorkbenchDialogs({
  info,
  prompts,
  archive,
  pendingChanges,
  startup,
  create,
  archivePayload,
  feedback
}: WorkbenchDialogsInput) {
  const infoDialogs = useWorkbenchInfoDialogs(info);
  const promptDialogs = useWorkbenchPromptDialogs({
    prompts,
    archive,
    pendingChanges,
    startup,
    create,
    archivePayload,
    feedback
  });

  return {
    archiveInfo: infoDialogs.archiveInfo,
    entryInfo: infoDialogs.entryInfo,
    ...promptDialogs
  };
}
