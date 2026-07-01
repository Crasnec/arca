import React from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  type Locale,
  type TranslateFn,
  SUPPORTED_LOCALES,
  currentLocale,
  readLocalePreference,
  translate,
  writeLocalePreference
} from "./messages";

type I18nContextValue = {
  locale: Locale;
  locales: typeof SUPPORTED_LOCALES;
  setLocale: (locale: Locale) => void;
  t: TranslateFn;
};

const I18nContext = React.createContext<I18nContextValue | null>(null);

export function I18nProvider({
  children,
  nativeMenu = true
}: {
  children: React.ReactNode;
  nativeMenu?: boolean;
}) {
  const [locale, setLocaleState] = React.useState<Locale>(currentLocale);

  const setLocale = React.useCallback((nextLocale: Locale) => {
    setLocaleState(nextLocale);
    writeLocalePreference(nextLocale);
  }, []);

  React.useEffect(() => {
    if (!nativeMenu) {
      return;
    }
    void invoke("set_native_menu_locale", { locale }).catch(() => undefined);
  }, [locale, nativeMenu]);

  const t = React.useCallback<TranslateFn>(
    (key, params) => translate(locale, key, params),
    [locale]
  );

  const value = React.useMemo<I18nContextValue>(
    () => ({
      locale,
      locales: SUPPORTED_LOCALES,
      setLocale,
      t
    }),
    [locale, setLocale, t]
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const value = React.useContext(I18nContext);
  if (!value) {
    throw new Error("useI18n must be used within I18nProvider");
  }
  return value;
}

export function getStoredLocale(): Locale {
  return readLocalePreference() ?? currentLocale();
}
