import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { currentLocale, translate } from "../i18n/messages";
import { ARCHIVE_DIALOG_FILTERS } from "../shared/constants";
import { currentAppSettings } from "../settings";

function title(key: Parameters<typeof translate>[1]) {
  return translate(currentLocale(), key);
}

function archiveDialogFilters() {
  return ARCHIVE_DIALOG_FILTERS;
}

export function chooseArchiveFile(defaultPath: string) {
  return openDialog({
    title: title("dialog.openArchive"),
    multiple: false,
    directory: false,
    defaultPath: defaultPath || undefined,
    filters: archiveDialogFilters()
  });
}

export function chooseExtractOutput(defaultPath: string) {
  return saveDialog({
    title: title("dialog.extractOutput"),
    defaultPath: defaultPath || undefined
  });
}

export function chooseExtractDirectory(defaultPath: string) {
  return openDialog({
    title: title("dialog.extractDestination"),
    directory: true,
    multiple: false,
    defaultPath: defaultPath || undefined,
    canCreateDirectories: true
  });
}

export function chooseCreateArchiveOutput(defaultPath: string) {
  const { defaultArchiveExtension } = currentAppSettings();
  return saveDialog({
    title: title("dialog.newArchive"),
    defaultPath: defaultPath || `archive.${defaultArchiveExtension}`,
    filters: archiveDialogFilters()
  });
}

export function chooseCreateInputFiles(multiple: boolean) {
  return openDialog({
    title: title("dialog.addFiles"),
    multiple,
    directory: false
  });
}

export function chooseCreateInputFolder() {
  return openDialog({
    title: title("dialog.addFolder"),
    multiple: false,
    directory: true,
    canCreateDirectories: false
  });
}

export function chooseDirectEditInputFiles() {
  return chooseCreateInputFiles(true);
}

export function chooseDirectEditInputFolder() {
  return chooseCreateInputFolder();
}
