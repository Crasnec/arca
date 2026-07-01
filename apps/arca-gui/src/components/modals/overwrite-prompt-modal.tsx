import type { OverwritePromptState } from "../../shared/types";
import { useI18n } from "../../i18n";
import styles from "./modals.module.css";

export type OverwritePromptModalProps = {
  prompt: OverwritePromptState | null;
  close: () => void;
  confirm: () => void;
};

export function OverwritePromptModal({ prompt, close, confirm }: OverwritePromptModalProps) {
  const { t } = useI18n();
  if (!prompt) {
    return null;
  }

  return (
    <div className={styles.backdrop} role="presentation">
      <div className={styles.modal} role="dialog" aria-label={t("overwrite.aria")}>
        <div className={styles.title}>{t("overwrite.title")}</div>
        <div className={styles.message}>{prompt.message}</div>
        <div className={styles.actions}>
          <button type="button" onClick={close}>
            {t("modal.cancel")}
          </button>
          <button type="button" onClick={confirm}>
            {prompt.action === "create" ? t("modal.replace") : t("modal.replaceAll")}
          </button>
        </div>
      </div>
    </div>
  );
}
