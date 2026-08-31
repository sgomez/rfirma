import type { i18n as I18n } from "i18next";
import { createContext, type ReactNode, useCallback, useContext, useMemo, useState } from "react";
import { I18nextProvider } from "react-i18next";
import { FALLBACK_LANGUAGE, isLanguageTag, type LanguageTag } from "./languages";
import type { LanguagePreference } from "./preference";

interface LanguageSelection {
  /** El idioma que se está enseñando. */
  language: LanguageTag;
  /** Cambia el idioma **en caliente** y lo guarda en la preferencia. */
  setLanguage: (language: LanguageTag) => Promise<void>;
}

const LanguageContext = createContext<LanguageSelection | null>(null);

interface LanguageProviderProps {
  i18n: I18n;
  preference: LanguagePreference;
  children: ReactNode;
}

/**
 * Enchufa i18next a React y deja el cambio de idioma al alcance de
 * Preferencias.
 *
 * El cambio se aplica sin reiniciar: `changeLanguage` avisa a i18next, los
 * componentes que usan `useTranslation` se repintan solos, y solo después se
 * guarda la preferencia. Ese orden importa —la interfaz responde aunque
 * guardar falle— y es el mismo «los cambios se aplican al hacerlos» de la
 * ficha de Preferencias: no hay «Guardar» ni «Cancelar».
 */
export function LanguageProvider({ i18n, preference, children }: LanguageProviderProps) {
  const [language, setLanguageState] = useState<LanguageTag>(() =>
    isLanguageTag(i18n.language) ? i18n.language : FALLBACK_LANGUAGE,
  );

  const setLanguage = useCallback(
    async (next: LanguageTag) => {
      await i18n.changeLanguage(next);
      setLanguageState(next);
      await preference.save(next);
    },
    [i18n, preference],
  );

  const selection = useMemo(() => ({ language, setLanguage }), [language, setLanguage]);

  return (
    <I18nextProvider i18n={i18n}>
      <LanguageContext.Provider value={selection}>{children}</LanguageContext.Provider>
    </I18nextProvider>
  );
}

/** El idioma actual y cómo cambiarlo. Solo dentro de `LanguageProvider`. */
export function useLanguage(): LanguageSelection {
  const selection = useContext(LanguageContext);
  if (!selection) {
    throw new Error("useLanguage fuera de LanguageProvider");
  }
  return selection;
}
