//! El `renderWithCatalog` de las pruebas: pinta un componente con el catálogo y el idioma enchufados.

import { type RenderResult, render as renderReact } from "@testing-library/react";
import { type ReactElement, type ReactNode, useMemo } from "react";
import { createI18n } from "../i18n/i18n";
import { LanguageProvider } from "../i18n/LanguageProvider";
import type { LanguageTag } from "../i18n/languages";
import { inMemoryLanguagePreference } from "../i18n/preference";

/** El proveedor de idioma de las pruebas y de Storybook: el catálogo real en el idioma pedido. */
export function CatalogProvider({
  language,
  children,
}: {
  language: LanguageTag;
  children: ReactNode;
}) {
  const i18n = useMemo(() => createI18n(language), [language]);
  return (
    <LanguageProvider i18n={i18n} preference={inMemoryLanguagePreference(language)}>
      {children}
    </LanguageProvider>
  );
}

/**
 * Pinta un componente con el catálogo enchufado, que es lo que necesita
 * cualquier prueba de interfaz: no hay ni una cadena escrita en línea, así que
 * sin `LanguageProvider` no habría texto que buscar.
 *
 * Vive fuera de los ficheros `*.test.tsx` a propósito: montar el proveedor a
 * mano en cada prueba es la clase de repetición que acaba divergiendo.
 */
export function renderWithCatalog(
  element: ReactElement,
  language: LanguageTag = "es",
): RenderResult {
  const wrapped = (inner: ReactNode) => (
    <CatalogProvider language={language}>{inner}</CatalogProvider>
  );
  const result = renderReact(wrapped(element));
  // `rerender` vuelve a envolver: el de `@testing-library` sustituye el árbol
  // entero por lo que se le pase, así que sin esto la segunda pintada perdería
  // el proveedor y no habría catálogo que leer.
  return { ...result, rerender: (next: ReactNode) => result.rerender(wrapped(next)) };
}
