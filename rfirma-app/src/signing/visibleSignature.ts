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

/**
 * El contenido del recuadro, por modelo (forma sacada del prototipo del
 * lienzo). La frase de *Personalizada* viaja **estructurada**, nunca como
 * comodines entre `$$…$$` (ADR-0006).
 */
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

/**
 * Lo que decide «Con rúbrica» y si *Solo rúbrica* se puede elegir, **en un
 * solo sitio** (docs/design/panel-de-firma.md § La rúbrica): *Solo rúbrica*
 * fuerza «Con rúbrica» encendido y bloqueado, y «Con rúbrica» apagado
 * desactiva *Solo rúbrica*.
 */
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
