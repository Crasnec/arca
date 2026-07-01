import React from "react";
import type { PasswordAction } from "../../shared/types";
import { useI18n } from "../../i18n";
import styles from "./modals.module.css";

export type PasswordPromptModalProps = {
  action: PasswordAction | null;
  archiveName: string;
  inputRef: React.RefObject<HTMLInputElement | null>;
  close: () => void;
  submit: (event: React.FormEvent) => void;
};

export function PasswordPromptModal({
  action,
  archiveName,
  inputRef,
  close,
  submit
}: PasswordPromptModalProps) {
  const { t } = useI18n();
  if (!action) {
    return null;
  }

  return (
    <div className={styles.backdrop} role="presentation">
      <form className={styles.modal} aria-label={t("passwordPrompt.aria")} onSubmit={submit}>
        <div className={styles.title}>{t("passwordPrompt.title")}</div>
        <div className={styles.subtitle}>{archiveName}</div>
        <label className={styles.fieldLabel} htmlFor="archive-password">
          {t("create.password")}
        </label>
        <input
          id="archive-password"
          ref={inputRef}
          aria-label={t("create.password")}
          className={styles.passwordInput}
          type="password"
          autoComplete="off"
        />
        <div className={styles.actions}>
          <button type="button" onClick={close}>
            {t("modal.cancel")}
          </button>
          <button type="submit">
            {action === "test" || action === "testSelection"
              ? t("modal.test")
              : t("modal.extract")}
          </button>
        </div>
      </form>
    </div>
  );
}
