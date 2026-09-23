import type { useTranslation } from "react-i18next";
import type { PageSet } from "../viewer/signatureBox";
import { sealedPages } from "../viewer/signatureBox";
import type { PageRangeError } from "./pageRange";

type Translate = ReturnType<typeof useTranslation>["t"];

/**
 * Lo que le pasa al campo: las situaciones del analizador, más **el campo
 * vacío**, que no es suya. `parsePageRange("")` es un `ok` con conjunto vacío
 * —el módulo es puro y ahí no hay nada que reprochar—, pero bajo «Estas
 * páginas» un campo sin páginas no puede firmar y hay que decirlo.
 */
export type FieldTrouble = PageRangeError | { kind: "empty" };

/** La cara del botón de sellar: qué dice y qué hace al pulsarlo. */
export interface SealButton {
  label: string;
  variant: string;
  act: () => void;
}

/** Cuántas páginas se nombran antes de pasar a contarlas. */
const ECHO_LIMIT = 6;

/**
 * La línea de eco bajo el campo: **qué páginas se van a sellar**, dichas una a
 * una (ID-98). Se nombran las seis primeras y el resto se cuenta, que es lo que
 * cabe en la columna más estrecha de la ventana.
 */
export function echoOf(pages: PageSet, pageCount: number, t: Translate): string | null {
  const list = sealedPages(pages, pageCount);
  if (list.length === 0) return null;
  const shown = list.slice(0, ECHO_LIMIT).join(", ");
  const rest = list.length - ECHO_LIMIT;
  return rest > 0
    ? t("panel.placement.echoMore", { pages: shown, count: rest })
    : t("panel.placement.echo", { pages: shown });
}

/**
 * La situación del campo, redactada. Es la vista quien la redacta y no el
 * analizador, que solo sabe qué ha pasado y no en qué idioma se cuenta (ID-29).
 */
export function messageFor(error: FieldTrouble, t: Translate): string {
  switch (error.kind) {
    case "empty":
      return t("panel.placement.errors.empty");
    case "beyond":
      return t("panel.placement.errors.beyond", {
        pageCount: error.pageCount,
        page: error.page,
      });
    case "reversed":
      return t("panel.placement.errors.reversed", { entry: error.entry });
    case "zero":
      return t("panel.placement.errors.zero");
    case "malformed":
      return t("panel.placement.errors.malformed", { entry: error.entry });
  }
}
