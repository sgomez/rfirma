//! La raíz de cualquier pantalla pintada fuera de la app: catálogo en un idioma, tema y la clase `rf-root`.

import { type ReactNode, useLayoutEffect, useMemo, useRef } from "react";
import { createI18n } from "../i18n/i18n";
import { LanguageProvider } from "../i18n/LanguageProvider";
import type { LanguageTag } from "../i18n/languages";
import { inMemoryLanguagePreference } from "../i18n/preference";

type Theme = "light" | "dark";

/** Envuelve Storybook y los diseños de Claude Design: sin ella no hay textos ni tokens. */
export function DesignRoot({
  language = "es",
  theme = "light",
  children,
}: {
  language?: LanguageTag;
  theme?: Theme;
  children: ReactNode;
}) {
  const i18n = useMemo(() => createI18n(language), [language]);
  useBodyAsPortalRoot(theme);
  return (
    <LanguageProvider i18n={i18n} preference={inMemoryLanguagePreference(language)}>
      <div className="rf-root" data-theme={theme}>
        {children}
      </div>
    </LanguageProvider>
  );
}

const mounted: { theme: Theme }[] = [];

/** Como `index.html`: los portales cuelgan de `body`; con varias raíces a la vez, `body` lleva el tema de la primera que sigue montada. */
function useBodyAsPortalRoot(theme: Theme) {
  const entry = useRef<{ theme: Theme }>({ theme });
  useLayoutEffect(() => {
    const own = entry.current;
    own.theme = theme;
    if (!mounted.includes(own)) mounted.push(own);
    paintBody();
  }, [theme]);
  useLayoutEffect(() => {
    const own = entry.current;
    return () => {
      mounted.splice(mounted.indexOf(own), 1);
      paintBody();
    };
  }, []);
}

function paintBody() {
  const { body } = document;
  const first = mounted[0];
  if (first === undefined) {
    body.classList.remove("rf-root");
    body.removeAttribute("data-theme");
    return;
  }
  body.classList.add("rf-root");
  body.setAttribute("data-theme", first.theme);
}
