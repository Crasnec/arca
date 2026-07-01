import { basename } from "./path-utils";
import { currentLocale, translate } from "../i18n/messages";

export function describeDropIntent(paths: string[]) {
  const locale = currentLocale();
  const firstPath = paths[0];
  if (!firstPath) {
    return translate(locale, "drop.openArchive");
  }
  const suffix =
    paths.length > 1
      ? translate(locale, "drop.ignoredSuffix", { count: paths.length - 1 })
      : "";
  return translate(locale, "drop.openPath", { name: basename(firstPath), suffix });
}

export function describeCreateDropIntent(paths: string[]) {
  const locale = currentLocale();
  const firstPath = paths[0];
  if (!firstPath) {
    return translate(locale, "drop.addInputs");
  }
  const suffix =
    paths.length > 1 ? translate(locale, "drop.moreSuffix", { count: paths.length - 1 }) : "";
  return translate(locale, "drop.addPath", { name: basename(firstPath), suffix });
}

export function describeDirectEditDropIntent(paths: string[]) {
  const locale = currentLocale();
  const firstPath = paths[0];
  if (!firstPath) {
    return translate(locale, "drop.addFiles");
  }
  const suffix =
    paths.length > 1 ? translate(locale, "drop.moreSuffix", { count: paths.length - 1 }) : "";
  return translate(locale, "drop.addPath", { name: basename(firstPath), suffix });
}
