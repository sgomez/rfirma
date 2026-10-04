//! Las situaciones del campo de páginas de «Varias», redactadas, y qué hace el botón que pone o quita la firma de la página. Sin React.

import type { useTranslation } from "react-i18next";
import { formatPageRange, type PageRangeError, parsePageRange } from "./pageRange";
import { type PageMode, type PageSet, type Placement, sealsPage } from "./pageSets";

type Translate = ReturnType<typeof useTranslation>["t"];

/**
 * Lo que le pasa al campo: las situaciones del analizador, más **el campo
 * vacío**, que no es suya. `parsePageRange("")` es un `ok` con conjunto vacío
 * —el módulo es puro y ahí no hay nada que reprochar—, pero bajo «Varias» un
 * campo sin páginas no puede firmar y hay que decirlo.
 */
export type FieldTrouble = PageRangeError | { kind: "empty" };

/** «Ponerla aquí» o «Quitarla de aquí». */
export type PageAction = "seal" | "unseal";

/** Lo que se escribe en el campo para un conjunto que llega de fuera. */
export function typedTextOf(pages: PageSet | null, pageCount: number): string {
  return pages === null ? "" : formatPageRange(pages, pageCount);
}

/** Lo tecleado, si nombra páginas: ni lo que no se entiende ni el campo vacío se aplican. */
export function typedPagesOf(text: string, pageCount: number): PageSet | null {
  const typed = parsePageRange(text, pageCount);
  return typed.ok ? typed.pages : null;
}

/** La situación del campo bajo el modo activo; fuera de «Varias» no hay ninguna. */
export function fieldTroubleOf(
  text: string,
  mode: PageMode,
  pageCount: number,
): FieldTrouble | null {
  if (mode !== "these") return null;
  if (text.trim() === "") return { kind: "empty" };
  const typed = parsePageRange(text, pageCount);
  return typed.ok ? null : typed.error;
}

/** Qué ofrece el botón de la página a la vista, o `null` si no hay botón. */
export function pageActionOf(
  placement: Placement | null,
  mode: PageMode,
  viewedPage: number,
  trouble: FieldTrouble | null,
): PageAction | null {
  const here = placement !== null && sealsPage(placement.pages, viewedPage);
  if (mode === "all" || (mode === "single" && here) || trouble !== null) return null;
  return here ? "unseal" : "seal";
}

/** La situación del campo, redactada en la frase corta del diseño. */
export function messageFor(error: FieldTrouble, t: Translate): string {
  switch (error.kind) {
    case "empty":
      return t("panel.placement.errors.empty");
    case "beyond":
      return t("panel.placement.errors.beyond", { count: error.pageCount });
    case "reversed":
      return t("panel.placement.errors.reversed", { entry: error.entry });
    case "zero":
      return t("panel.placement.errors.zero");
    case "malformed":
      return t("panel.placement.errors.malformed");
  }
}
