//! La raíz de cualquier pantalla pintada fuera de la app: catálogo en un idioma, tema y la clase `rf-root`.

import { type ReactNode, useLayoutEffect, useMemo } from "react";
import { createI18n } from "../i18n/i18n";
import { LanguageProvider } from "../i18n/LanguageProvider";
import type { LanguageTag } from "../i18n/languages";
import { inMemoryLanguagePreference } from "../i18n/preference";

/** Envuelve Storybook y los diseños de Claude Design: sin ella no hay textos ni tokens. */
export function DesignRoot({
  language = "es",
  theme = "light",
  children,
}: {
  language?: LanguageTag;
  theme?: "light" | "dark";
  children: ReactNode;
}) {
  const i18n = useMemo(() => createI18n(language), [language]);
  useBodyAsRoot(theme);
  return (
    <LanguageProvider i18n={i18n} preference={inMemoryLanguagePreference(language)}>
      <div className="rf-root" data-theme={theme}>
        {children}
      </div>
    </LanguageProvider>
  );
}

/** Como `index.html`: los portales cuelgan de `body`, y sin `rf-root` ni tema allí salen sin fuente ni colores. */
function useBodyAsRoot(theme: "light" | "dark") {
  useLayoutEffect(() => {
    const { body } = document;
    body.classList.add("rf-root");
    body.setAttribute("data-theme", theme);
    return () => {
      body.classList.remove("rf-root");
      body.removeAttribute("data-theme");
    };
  }, [theme]);
}
