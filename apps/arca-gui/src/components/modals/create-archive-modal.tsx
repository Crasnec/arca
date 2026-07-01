import type React from "react";
import {
  CreateArchiveEncryptionField,
  CreateArchiveInputActions,
  CreateArchiveInputList,
  CreateArchiveOutputField
} from "./create-archive-fields";
import type { DropState } from "../../shared/types";
import { useI18n } from "../../i18n";
import styles from "./modals.module.css";

export type CreateArchiveModalProps = {
  open: boolean;
  dropState: DropState;
  outputPath: string;
  loading: boolean;
  singleStreamOutput: boolean;
  inputs: string[];
  encryptionAllowed: boolean;
  encrypt: boolean;
  canCreate: boolean;
  passwordInputRef: React.RefObject<HTMLInputElement | null>;
  setOutputPath: (value: string) => void;
  setEncrypt: (value: boolean) => void;
  chooseOutput: () => void | Promise<void>;
  addFiles: () => void | Promise<void>;
  addFolder: () => void | Promise<void>;
  removeInput: (path: string) => void;
  close: () => void;
  create: (overwrite?: boolean) => void | Promise<void>;
};

export function CreateArchiveModal({
  open,
  dropState,
  outputPath,
  loading,
  singleStreamOutput,
  inputs,
  encryptionAllowed,
  encrypt,
  canCreate,
  passwordInputRef,
  setOutputPath,
  setEncrypt,
  chooseOutput,
  addFiles,
  addFolder,
  removeInput,
  close,
  create
}: CreateArchiveModalProps) {
  const { t } = useI18n();
  if (!open) {
    return null;
  }

  return (
    <div className={styles.backdrop} role="presentation">
      <form
        className={`${styles.modal} ${styles.createModal}${dropState === "hover" ? ` ${styles.dropTarget}` : ""}`}
        aria-label={t("create.title")}
        onSubmit={(event) => {
          event.preventDefault();
          void create();
        }}
      >
        <div className={styles.title}>{t("create.title")}</div>
        <CreateArchiveOutputField
          outputPath={outputPath}
          loading={loading}
          setOutputPath={setOutputPath}
          chooseOutput={chooseOutput}
        />
        <CreateArchiveInputActions
          loading={loading}
          singleStreamOutput={singleStreamOutput}
          addFiles={addFiles}
          addFolder={addFolder}
        />
        <CreateArchiveInputList inputs={inputs} loading={loading} removeInput={removeInput} />
        <CreateArchiveEncryptionField
          loading={loading}
          encryptionAllowed={encryptionAllowed}
          encrypt={encrypt}
          passwordInputRef={passwordInputRef}
          setEncrypt={setEncrypt}
        />
        <div className={styles.actions}>
          <button type="button" disabled={loading} onClick={close}>
            {t("modal.cancel")}
          </button>
          <button type="submit" disabled={!canCreate}>
            {t("modal.create")}
          </button>
        </div>
      </form>
    </div>
  );
}
