import type { ArchiveManifest, ArchiveValidation } from "./types";
import { translate, type TranslateFn } from "../i18n/messages";

const fallbackT: TranslateFn = (key, params) => translate("en", key, params);

export function isSingleStreamManifest(manifest: ArchiveManifest | null) {
  return manifest ? ["gz", "bz2", "xz"].includes(manifest.formatKind) : false;
}

export function archiveStatusLabel(
  validation: ArchiveValidation,
  hasPendingChanges: boolean,
  t: TranslateFn = fallbackT
) {
  if (hasPendingChanges) {
    return t("validation.unsavedChanges");
  }
  if (validation.fullyValidated) {
    return t("validation.tested");
  }
  if (validation.passwordRequired) {
    return t("validation.passwordRequired");
  }
  if (validation.payloadValidated) {
    return t("validation.tested");
  }
  if (validation.metadataValidated) {
    return t("validation.notTested");
  }
  return t("validation.ready");
}

export function archiveValidationTitle(
  validation: ArchiveValidation,
  hasPendingChanges: boolean,
  t: TranslateFn = fallbackT
) {
  if (hasPendingChanges) {
    return t("validation.saveBeforeTest");
  }
  if (validation.fullyValidated || validation.payloadValidated) {
    return t("validation.contentsTested");
  }
  if (validation.passwordRequired) {
    return t("validation.passwordRequiredTitle");
  }
  if (validation.metadataValidated) {
    return t("validation.metadataOnlyTitle");
  }
  return t("validation.ready");
}

export function archiveManifestFullyValidated(
  manifest: ArchiveManifest,
  reason: string
): ArchiveManifest {
  return {
    ...manifest,
    validation: {
      ...manifest.validation,
      payloadValidated: true,
      fullyValidated: true,
      state: manifest.validation.passwordRequired
        ? "fullyValidatedPasswordRequired"
        : "fullyValidated",
      reason
    }
  };
}
