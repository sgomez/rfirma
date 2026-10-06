//! Qué se estampa en el recuadro de la firma visible: el modelo, la frase de *Personalizada*, la regla de «Con rúbrica» y la última configuración recordada.
/**
 * **Modelo, no comodines**. El usuario no escribe `$$SUBJECTCN$$` ni
 * `$$SIGNDATE$$`: elige uno de los tres modelos, y el texto lo compone Rust en
 * `signing::layer2_text` con las etiquetas y la máscara del DNI de AutoFirma.
 * Aquí no hay ni una cadena del recuadro: si te encuentras escribiendo
 * «Firmado por» o una máscara de asteriscos en este directorio, estás
 * duplicando ese módulo.
 */

/** Un dato del certificado, insertable en la frase de *Personalizada*. */
export type Datum = "signer" | "issuer" | "signedAt";

/** Un trozo de la frase de *Personalizada*: texto libre o un dato. */
export type PhrasePart = { text: string } | { datum: Datum };

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

/** La sección de la firma visible: lo que se estampa y cómo se cambia. */
export interface VisibleSignatureSection {
  value: VisibleSignature;
  change: (signature: VisibleSignature) => void;
}

/**
 * Lo que sale marcado la primera vez: recuadro no —firmar sin él está
 * permitido, y encenderlo es un gesto aparte—, y dentro, para cuando se
 * encienda, el modelo *Completa*, sin rúbrica porque todavía no hay imagen.
 */
export const DEFAULT_VISIBLE_SIGNATURE: VisibleSignature = {
  enabled: false,
  withRubric: false,
  content: { model: "complete" },
};

/** Modelo, frase y «Con rúbrica» de la última firma visible configurada, o nada la primera vez (ADR-0010). */
export interface RememberedVisibleSignature {
  content: VisibleContent | null;
  withRubric: boolean;
}

/** Por dónde entra la memoria global de firma visible; se lee una vez al arrancar. */
export interface VisibleSignatureMemory {
  read(): Promise<RememberedVisibleSignature>;
}

/** El recuadro apagado con lo último recordado; sin memoria, `DEFAULT_VISIBLE_SIGNATURE`. */
export function visibleSignatureFrom(remembered: RememberedVisibleSignature): VisibleSignature {
  return {
    enabled: false,
    withRubric: remembered.withRubric,
    content: remembered.content ?? DEFAULT_VISIBLE_SIGNATURE.content,
  };
}

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
