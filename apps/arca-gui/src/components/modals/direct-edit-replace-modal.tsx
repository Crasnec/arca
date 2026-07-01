import type { DirectEditReplacePromptState } from "../../shared/types";
import { replacementPromptMessage } from "../../shared/prompt-utils";
import { useI18n } from "../../i18n";
import styles from "./modals.module.css";

export type DirectEditReplaceModalProps = {
  prompt: DirectEditReplacePromptState | null;
  close: () => void;
  skip: () => void;
  replace: () => void;
  skipAll: () => void;
  replaceAll: () => void;
};

export function DirectEditReplaceModal({
  prompt,
  close,
  skip,
  replace,
  skipAll,
  replaceAll
}: DirectEditReplaceModalProps) {
  const { t } = useI18n();
  if (!prompt) {
    return null;
  }

  return (
    <div className={styles.backdrop} role="presentation">
      <div
        className={`${styles.modal} ${styles.conflictModal}`}
        role="dialog"
        aria-label={t("directEditReplace.aria")}
      >
        <div className={styles.title}>{t("directEditReplace.title")}</div>
        <div className={styles.message}>{replacementPromptMessage(prompt, t)}</div>
        <div className={styles.actions}>
          <button type="button" onClick={close}>
            {t("modal.cancel")}
          </button>
          <button type="button" onClick={skip}>
            {t("modal.skip")}
          </button>
          <button type="button" onClick={replace}>
            {t("modal.replace")}
          </button>
          <button type="button" onClick={skipAll}>
            {t("modal.skipAll")}
          </button>
          <button type="button" onClick={replaceAll}>
            {t("modal.replaceAll")}
          </button>
        </div>
      </div>
    </div>
  );
}
