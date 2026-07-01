import type { EntrySortState } from "./types";

export const SUPPORTED_ARCHIVE_EXTENSIONS = [
  "zip",
  "tar",
  "tar.gz",
  "tgz",
  "tar.bz2",
  "tbz2",
  "tar.xz",
  "txz",
  "gz",
  "bz2",
  "xz"
] as const;

export const CREATE_ARCHIVE_EXTENSIONS = [
  "zip",
  "tar",
  "tar.gz",
  "tgz",
  "tar.bz2",
  "tbz2",
  "tar.xz",
  "txz"
] as const;

export const ARCHIVE_DIALOG_FILTERS = [
  {
    name: "Archives",
    extensions: [...SUPPORTED_ARCHIVE_EXTENSIONS]
  }
];

export const PENDING_HISTORY_LIMIT = 24;
export const OPERATION_PROGRESS_EVENT = "arca-operation-progress";
export const CLOSE_BLOCKED_EVENT = "arca-close-blocked";
export const STARTUP_REQUESTS_EVENT = "arca-startup-requests";
export const MENU_ACTION_EVENT = "arca-menu-action";
export const OPEN_SETTINGS_EVENT = "arca-open-settings";
export const DEFAULT_ENTRY_SORT: EntrySortState = { column: "path", direction: "asc" };
