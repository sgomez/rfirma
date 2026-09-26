import type { useTranslation } from "react-i18next";
import type { PageRangeError } from "./pageRange";

type Translate = ReturnType<typeof useTranslation>["t"];

/**
 * Lo que le pasa al campo: las situaciones del analizador, más **el campo
 * vacío**, que no es suya. `parsePageRange("")` es un `ok` con conjunto vacío
 * —el módulo es puro y ahí no hay nada que reprochar—, pero bajo «Varias» un
 * campo sin páginas no puede firmar y hay que decirlo.
 */
export type FieldTrouble = PageRangeError | { kind: "empty" };

/** «Ponerla aquí» o «Quitarla de aquí»: qué dice y qué hace al pulsarlo. */
export interface PageButton {
  label: string;
  act: () => void;
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
