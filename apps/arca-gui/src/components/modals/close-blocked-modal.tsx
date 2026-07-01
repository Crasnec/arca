import type { CloseBlockedPromptState } from "../../shared/types";
import { useI18n } from "../../i18n";
import styles from "./modals.module.css";

export type CloseBlockedModalProps = {
  prompt: CloseBlockedPromptState | null;
  close: () => void;
};

export function CloseBlockedModal({ prompt, close }: CloseBlockedModalProps) {
  const { t } = useI18n();
  if (!prompt) {
    return null;
  }

  return (
    <div className={styles.backdrop} role="presentation">
      <div className={styles.modal} role="dialog" aria-label={t("closeBlocked.aria")}>
        <div className={styles.title}>{t("closeBlocked.title")}</div>
        <div className={styles.message}>
          {prompt.activeLabels.length > 1
            ? `${prompt.message}\n${prompt.activeLabels.join("\n")}`
            : prompt.message}
        </div>
        <div className={styles.actions}>
          <button type="button" onClick={close}>
            {t("modal.ok")}
          </button>
        </div>
      </div>
    </div>
  );
}
