import type { Placement } from "../viewer/signatureBox";
import { sealedPages } from "../viewer/signatureBox";

/**
 * Qué se estampa en el recuadro de la firma visible.
 *
 * **Modelo, no comodines** (ID-19). El usuario no escribe `$$SUBJECTCN$$` ni
 * `$$SIGNDATE$$`: elige uno de los tres modelos, y el texto lo compone Rust en
 * `signing::layer2_text` con las etiquetas y la máscara del DNI de AutoFirma.
 * Aquí no hay ni una cadena del recuadro: si te encuentras escribiendo
 * «Firmado por» o una máscara de asteriscos en este directorio, estás
 * duplicando ese módulo.
 */

/** Un dato del certificado, insertable en la frase de *Personalizada*. */
type Datum = "signer" | "issuer" | "signedAt";

/** Un trozo de la frase de *Personalizada*: texto libre o un dato. */
type PhrasePart = { text: string } | { datum: Datum };

/** El contenido del recuadro, por modelo; la frase de *Personalizada* viaja estructurada, no como comodines (ADR-0006). */
export type VisibleContent =
  | { model: "complete" }
  | { model: "rubricOnly" }
  | { model: "custom"; phrase: PhrasePart[] };

/** Todo lo que decide el recuadro, tal como lo deja el panel. */
export interface VisibleSignature {
  /** Si se estampa recuadro. Apagado, el PDF se firma sin nada visible. */
  enabled: boolean;
  /** Si la rúbrica va dentro del recuadro. Común a los tres modelos. */
  withRubric: boolean;
  content: VisibleContent;
}

/**
 * Lo que sale marcado la primera vez: recuadro no —firmar sin él está
 * permitido, y encenderlo es un gesto aparte (#974)—, y dentro, para cuando se
 * encienda, el modelo *Completa*, sin rúbrica porque todavía no hay imagen.
 */
export const DEFAULT_VISIBLE_SIGNATURE: VisibleSignature = {
  enabled: false,
  withRubric: false,
  content: { model: "complete" },
};

/** Lo que deciden juntos «Con rúbrica» y si *Solo rúbrica* se puede elegir. */
export interface RubricRule {
  /** Si el interruptor «Con rúbrica» está bloqueado, y en qué sentido. */
  locked: "on" | null;
  /** Si la tarjeta *Solo rúbrica* se puede elegir. */
  rubricOnlySelectable: boolean;
}

export function rubricRuleFor(content: VisibleContent, withRubric: boolean): RubricRule {
  if (content.model === "rubricOnly") return { locked: "on", rubricOnlySelectable: true };
  return { locked: null, rubricOnlySelectable: withRubric };
}

/** El hueco punteado de la rúbrica encendida sin imagen: al lado del texto, o llenando el recuadro. */
export type RubricGap = "beside" | "fill";

export function rubricGapFor(signature: VisibleSignature, hasRubric: boolean): RubricGap | null {
  if (hasRubric) return null;
  if (signature.content.model === "rubricOnly") return "fill";
  return signature.withRubric ? "beside" : null;
}

/**
 * La línea de solo lectura del resumen, tras firmar
 * (docs/design/panel-de-firma.md § El resumen, tras firmar): «No», «En la
 * página N», «En todas las páginas» o «En N de M páginas».
 */
export type VisibleSignaturePlacement =
  | { kind: "none" }
  | { kind: "onPage"; page: number }
  | { kind: "allPages" }
  | { kind: "somePages"; sealed: number; total: number };

/**
 * `pageCount` es `null` cuando el recuento del documento no se conoce
 * todavía: sin él no se puede decir «de M páginas», así que la línea entera
 * no se compone —no se inventa un total—.
 */
export function summarizeVisiblePlacement(
  enabled: boolean,
  placement: Placement | null,
  pageCount: number | null,
): VisibleSignaturePlacement | null {
  if (!enabled || placement === null) return { kind: "none" };
  if (pageCount === null) return null;
  if (placement.pages === "all") return { kind: "allPages" };
  const pages = sealedPages(placement.pages, pageCount);
  const [only] = pages;
  if (pages.length === 1 && only !== undefined) return { kind: "onPage", page: only };
  return { kind: "somePages", sealed: pages.length, total: pageCount };
}
