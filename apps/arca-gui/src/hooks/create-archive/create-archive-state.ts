import React from "react";
import { isSingleStreamOutputPath, isZipOutputPath } from "../../shared/path-utils";

type AppendCreateInputsResult =
  | { kind: "cancelled" }
  | { kind: "rejected"; message: string }
  | { kind: "added"; count: number };
type CreatePasswordResult =
  | { kind: "ready"; password?: string }
  | { kind: "rejected"; message: string };

export function useCreateArchiveState({ loading }: { loading: boolean }) {
  const [createOpen, setCreateOpen] = React.useState(false);
  const [createOutputPath, setCreateOutputPath] = React.useState("");
  const [createInputs, setCreateInputs] = React.useState<string[]>([]);
  const [createEncrypt, setCreateEncrypt] = React.useState(false);
  const createPasswordInputRef = React.useRef<HTMLInputElement | null>(null);

  const createEncryptionAllowed = isZipOutputPath(createOutputPath);
  const createSingleStreamOutput = isSingleStreamOutputPath(createOutputPath);
  const createSingleStreamInputLimitExceeded = createSingleStreamOutput && createInputs.length > 1;
  const canCreateArchive =
    createOutputPath.trim() !== "" &&
    createInputs.length > 0 &&
    !createSingleStreamInputLimitExceeded &&
    !loading;

  React.useEffect(() => {
    if (createEncryptionAllowed || !createEncrypt) {
      return;
    }
    if (createPasswordInputRef.current) {
      createPasswordInputRef.current.value = "";
    }
    setCreateEncrypt(false);
  }, [createEncryptionAllowed, createEncrypt]);

  function showCreateModal() {
    setCreateOpen(true);
  }

  function closeCreateModal() {
    if (createPasswordInputRef.current) {
      createPasswordInputRef.current.value = "";
    }
    setCreateOpen(false);
  }

  function resetCreateForm() {
    if (createPasswordInputRef.current) {
      createPasswordInputRef.current.value = "";
    }
    setCreateOpen(false);
    setCreateInputs([]);
    setCreateOutputPath("");
    setCreateEncrypt(false);
  }

  function appendCreateInputs(selected: string | string[] | null): AppendCreateInputsResult {
    if (!selected) {
      return { kind: "cancelled" };
    }
    const paths = Array.isArray(selected) ? selected : [selected];
    if (paths.length === 0) {
      return { kind: "cancelled" };
    }
    const nextInputs = [...new Set([...createInputs, ...paths])];
    if (createSingleStreamOutput && nextInputs.length > 1) {
      return {
        kind: "rejected",
        message: "Single-stream outputs require exactly one file input"
      };
    }
    setCreateInputs(nextInputs);
    return { kind: "added", count: paths.length };
  }

  function removeCreateInput(path: string) {
    setCreateInputs((current) => current.filter((input) => input !== path));
  }

  function readCreatePassword(): CreatePasswordResult {
    const password = createEncrypt ? createPasswordInputRef.current?.value ?? "" : undefined;
    if (createEncrypt && !createEncryptionAllowed) {
      if (createPasswordInputRef.current) {
        createPasswordInputRef.current.value = "";
      }
      setCreateEncrypt(false);
      return { kind: "rejected", message: "Password is only available for ZIP archives" };
    }
    if (createEncrypt && !password) {
      createPasswordInputRef.current?.focus();
      return { kind: "rejected", message: "Password is required for encrypted ZIP creation" };
    }
    if (createPasswordInputRef.current) {
      createPasswordInputRef.current.value = "";
    }
    return { kind: "ready", password };
  }

  return {
    createOpen,
    createOutputPath,
    createInputs,
    createEncrypt,
    createPasswordInputRef,
    createEncryptionAllowed,
    createSingleStreamOutput,
    createSingleStreamInputLimitExceeded,
    canCreateArchive,
    setCreateOutputPath,
    setCreateEncrypt,
    showCreateModal,
    closeCreateModal,
    resetCreateForm,
    appendCreateInputs,
    removeCreateInput,
    readCreatePassword
  };
}
