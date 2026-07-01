import { invoke } from "@tauri-apps/api/core";

export type ArchiveFormatCapability = {
  id: string;
  name: string;
  description: string;
  suffixes: string[];
  createSuffixes: string[];
  extensions: string[];
  mimeType: string;
  supportsCreate: boolean;
  supportsExtract: boolean;
  supportsTest: boolean;
  supportsDirectEdit: boolean;
  signatures: Array<{
    offset: number;
    bytesHex: string;
  }>;
};

export type FileAssociationEntry = {
  extension: string;
  enabled: boolean;
  defaultHandler: boolean;
  registeredHandler: boolean;
  openWithHandler: boolean;
  openCommand: boolean;
  extractCommand: boolean;
  testCommand: boolean;
};

export type FileAssociationStatus = {
  supported: boolean;
  entries: FileAssociationEntry[];
  message: string | null;
};

export function archiveFormatCapabilities() {
  return invoke<ArchiveFormatCapability[]>("archive_format_capabilities");
}

export function fileAssociationStatus() {
  return invoke<FileAssociationStatus>("file_association_status");
}

export function setNativeFileAssociation(extension: string, enabled: boolean) {
  return invoke<FileAssociationEntry>("set_file_association", { extension, enabled });
}

export function setNativeFileAssociations(enabled: boolean) {
  return invoke<FileAssociationStatus>("set_all_file_associations", { enabled });
}
