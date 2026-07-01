import { invoke } from "@tauri-apps/api/core";
import type {
  ArchiveManifest,
  CreateResult,
  DirectEditAddPlan,
  DirectEditPlannedEntry,
  ExtractResult
} from "../shared/types";

export function listArchive(archivePath: string, operationId: number) {
  return invoke<ArchiveManifest>("list_archive", { archivePath, operationId });
}

export function testArchivePayloads({
  archivePath,
  password,
  operationId
}: {
  archivePath: string;
  password?: string;
  operationId: number;
}) {
  return invoke("test_archive", { archivePath, password, operationId });
}

export function testSelectedArchiveEntries({
  archivePath,
  entries,
  password,
  operationId
}: {
  archivePath: string;
  entries: string[];
  password?: string;
  operationId: number;
}) {
  return invoke("test_selected_entries", { archivePath, entries, password, operationId });
}

export function extractArchivePayloads({
  archivePath,
  outputPath,
  password,
  overwrite,
  operationId
}: {
  archivePath: string;
  outputPath: string;
  password?: string;
  overwrite: boolean;
  operationId: number;
}) {
  return invoke<ExtractResult>("extract_archive", {
    archivePath,
    outputPath,
    password,
    overwrite,
    operationId
  });
}

export function extractSelectedArchiveEntries({
  archivePath,
  outputPath,
  entries,
  password,
  overwrite,
  operationId
}: {
  archivePath: string;
  outputPath: string;
  entries: string[];
  password?: string;
  overwrite: boolean;
  operationId: number;
}) {
  return invoke<ExtractResult>("extract_selected_entries", {
    request: {
      archivePath,
      outputPath,
      entries,
      password,
      overwrite
    },
    operationId
  });
}

export function createArchiveCommand({
  outputPath,
  inputs,
  password,
  overwrite,
  operationId
}: {
  outputPath: string;
  inputs: string[];
  password?: string;
  overwrite: boolean;
  operationId: number;
}) {
  return invoke<CreateResult>("create_archive", {
    outputPath,
    inputs,
    password,
    overwrite,
    operationId
  });
}

export function planDirectEditAddCommand({
  archivePath,
  inputs,
  pendingDeleteEntries,
  pendingAddEntries,
  operationId
}: {
  archivePath: string;
  inputs: string[];
  pendingDeleteEntries: string[];
  pendingAddEntries: DirectEditPlannedEntry[];
  operationId: number;
}) {
  return invoke<DirectEditAddPlan>("plan_direct_edit_add", {
    archivePath,
    inputs,
    pendingDeleteEntries,
    pendingAddEntries,
    operationId
  });
}

export function saveDirectEditCommand({
  archivePath,
  expectedDigestSha256,
  deleteEntries,
  addInputs,
  addEntries,
  replaceEntries,
  operationId
}: {
  archivePath: string;
  expectedDigestSha256: string;
  deleteEntries: string[];
  addInputs: string[];
  addEntries: string[];
  replaceEntries: string[];
  operationId: number;
}) {
  return invoke<ArchiveManifest>("save_direct_edit", {
    request: {
      archivePath,
      expectedDigestSha256,
      deleteEntries,
      addInputs,
      addEntries,
      replaceEntries
    },
    operationId
  });
}
