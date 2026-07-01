import type { UnsavedPromptState } from "../../shared/types";
import { useI18n } from "../../i18n";
import styles from "./modals.module.css";

export type UnsavedChangesModalProps = {
  prompt: UnsavedPromptState | null;
  close: () => void;
  discard: () => void;
};

export function UnsavedChangesModal({ prompt, close, discard }: UnsavedChangesModalProps) {
  const { t } = useI18n();
  if (!prompt) {
    return null;
  }

  return (
    <div className={styles.backdrop} role="presentation">
      <div className={styles.modal} role="dialog" aria-label={t("unsaved.aria")}>
        <div className={styles.title}>{t("unsaved.title")}</div>
        <div className={styles.message}>{prompt.message}</div>
        <div className={styles.actions}>
          <button type="button" onClick={close}>
            {t("modal.cancel")}
          </button>
          <button type="button" onClick={discard}>
            {t("modal.discard")}
          </button>
        </div>
      </div>
    </div>
  );
}
