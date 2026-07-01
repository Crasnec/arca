import type React from "react";
import { FileArchive, FolderOpen, Save, Trash2 } from "lucide-react";
import { useI18n } from "../../i18n";
import styles from "./modals.module.css";

export type CreateArchiveOutputFieldProps = {
  outputPath: string;
  loading: boolean;
  setOutputPath: (value: string) => void;
  chooseOutput: () => void | Promise<void>;
};

export function CreateArchiveOutputField({
  outputPath,
  loading,
  setOutputPath,
  chooseOutput
}: CreateArchiveOutputFieldProps) {
  const { t } = useI18n();
  return (
    <div className={styles.field}>
      <label className={styles.fieldLabel} htmlFor="create-archive-output">
        {t("create.saveAs")}
      </label>
      <div className={styles.fieldRow}>
        <input
          id="create-archive-output"
          aria-label={t("create.archiveOutput")}
          className={styles.pathInput}
          value={outputPath}
          onChange={(event) => setOutputPath(event.target.value)}
          placeholder={t("create.outputFile")}
          spellCheck={false}
          disabled={loading}
        />
        <button
          type="button"
          title={t("create.chooseOutputFile")}
          disabled={loading}
          onClick={() => void chooseOutput()}
        >
          <Save size={16} aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}

export type CreateArchiveInputActionsProps = {
  loading: boolean;
  singleStreamOutput: boolean;
  addFiles: () => void | Promise<void>;
  addFolder: () => void | Promise<void>;
};

export function CreateArchiveInputActions({
  loading,
  singleStreamOutput,
  addFiles,
  addFolder
}: CreateArchiveInputActionsProps) {
  const { t } = useI18n();
  return (
    <div className={styles.field}>
      <div className={styles.fieldLabel}>{t("create.files")}</div>
      <div className={styles.fieldRow}>
        <button type="button" disabled={loading} onClick={() => void addFiles()}>
          <FileArchive size={16} aria-hidden="true" />
          <span>{t("create.addFiles")}</span>
        </button>
        <button
          type="button"
          title={singleStreamOutput ? t("create.onlyOneFile") : t("create.addFolder")}
          disabled={loading || singleStreamOutput}
          onClick={() => void addFolder()}
        >
          <FolderOpen size={16} aria-hidden="true" />
          <span>{t("create.addFolder")}</span>
        </button>
      </div>
    </div>
  );
}

export type CreateArchiveInputListProps = {
  inputs: string[];
  loading: boolean;
  removeInput: (path: string) => void;
};

export function CreateArchiveInputList({
  inputs,
  loading,
  removeInput
}: CreateArchiveInputListProps) {
  const { t } = useI18n();
  return (
    <div className={styles.inputList} aria-label={t("create.archiveInputs")}>
      {inputs.length === 0 ? (
        <div className={styles.inputListEmpty}>{t("create.noFilesSelected")}</div>
      ) : (
        inputs.map((input) => (
          <div className={styles.inputListRow} key={input}>
            <span>{input}</span>
            <button
              type="button"
              title={t("create.removeInput")}
              aria-label={t("create.removeInput")}
              disabled={loading}
              onClick={() => removeInput(input)}
            >
              <Trash2 size={15} aria-hidden="true" />
            </button>
          </div>
        ))
      )}
    </div>
  );
}

export type CreateArchiveEncryptionFieldProps = {
  loading: boolean;
  encryptionAllowed: boolean;
  encrypt: boolean;
  passwordInputRef: React.RefObject<HTMLInputElement | null>;
  setEncrypt: (value: boolean) => void;
};

export function CreateArchiveEncryptionField({
  loading,
  encryptionAllowed,
  encrypt,
  passwordInputRef,
  setEncrypt
}: CreateArchiveEncryptionFieldProps) {
  const { t } = useI18n();
  return (
    <div className={styles.field}>
      <label
        className={styles.checkboxRow}
        title={encryptionAllowed ? t("create.protectTitle") : t("create.passwordZipOnlyTitle")}
      >
        <input
          type="checkbox"
          checked={encrypt}
          disabled={loading || !encryptionAllowed}
          onChange={(event) => setEncrypt(event.target.checked)}
        />
        <span>{t("create.passwordProtect")}</span>
      </label>
      {encrypt && (
        <div className={styles.passwordField}>
          <label className={styles.fieldLabel} htmlFor="create-archive-password">
            {t("create.password")}
          </label>
          <input
            id="create-archive-password"
            ref={passwordInputRef}
            aria-label={t("create.createPassword")}
            className={styles.passwordInput}
            type="password"
            autoComplete="off"
            disabled={loading}
          />
        </div>
      )}
    </div>
  );
}
