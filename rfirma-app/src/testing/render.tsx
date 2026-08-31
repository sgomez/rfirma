import { type RenderResult, render as renderReact } from "@testing-library/react";
import type { ReactElement } from "react";
import { createI18n } from "../i18n/i18n";
import { LanguageProvider } from "../i18n/LanguageProvider";
import type { LanguageTag } from "../i18n/languages";
import { inMemoryLanguagePreference } from "../i18n/preference";

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
  return renderReact(
    <LanguageProvider i18n={createI18n(language)} preference={inMemoryLanguagePreference(language)}>
      {element}
    </LanguageProvider>,
  );
}
