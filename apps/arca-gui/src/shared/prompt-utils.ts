import type { DirectEditReplacePromptState } from "./types";
import { translate, type TranslateFn } from "../i18n/messages";

const fallbackT: TranslateFn = (key, params) => translate("en", key, params);

export function replacementPromptMessage(
  prompt: DirectEditReplacePromptState,
  t: TranslateFn = fallbackT
) {
  const current = prompt.plan.replacements[0];
  if (!current) {
    return t("directEditReplace.noConflicts");
  }
  const remaining = prompt.plan.replacements.length - 1;
  const accepted = prompt.acceptedReplacements.length;
  const lines = [t("directEditReplace.exists", { path: current.archivePath })];
  if (remaining > 0) {
    lines.push(
      t("directEditReplace.moreConflicts", {
        count: remaining,
        plural: remaining === 1 ? "" : "s"
      })
    );
  }
  if (accepted > 0) {
    lines.push(
      t("directEditReplace.selected", {
        count: accepted,
        plural: accepted === 1 ? "" : "s"
      })
    );
  }
  return lines.join("\n");
}
