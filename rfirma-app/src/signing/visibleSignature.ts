/**
 * Qué se estampa en el recuadro de la firma visible.
 *
 * **Casillas, no comodines** (ID-19). El usuario nunca escribe `$$SUBJECTCN$$`
 * ni `$$SIGNDATE$$`: marca qué dato aparece, y el texto lo compone Rust en
 * `signing::layer2_text` con las etiquetas y la máscara del DNI de AutoFirma.
 * Aquí no hay ni una cadena del recuadro: si te encuentras escribiendo
 * «Firmado por» o una máscara de asteriscos en este directorio, estás
 * duplicando ese módulo.
 */

/** Las cuatro casillas de texto. La rúbrica va aparte: es una imagen. */
export interface VisibleTextFields {
  signerName: boolean;
  issuer: boolean;
  signedAt: boolean;
  reason: boolean;
}

/** Todo lo que decide el recuadro, tal como lo deja el panel. */
export interface VisibleSignature {
  /** Si se estampa recuadro. Apagado, el PDF se firma sin nada visible. */
  enabled: boolean;
  /** Si la rúbrica va dentro del recuadro. Sin imagen elegida, no se puede. */
  rubric: boolean;
  fields: VisibleTextFields;
  /** El motivo que escribe el usuario. Vacío es «sin motivo». */
  reason: string;
}

/**
 * Lo que sale marcado la primera vez: recuadro sí, y dentro el firmante —con
 * el DNI ya dentro del nombre (ADR-0006 v0.3.1)— y la fecha, que es el
 * contenido de un recuadro administrativo corriente. El emisor no, por ser
 * un dato añadido y no el que se venía mostrando; la rúbrica no, porque
 * todavía no hay imagen; el motivo tampoco, porque está vacío y una etiqueta
 * «Motivo:» sin nada detrás no dice nada.
 */
export const DEFAULT_VISIBLE_SIGNATURE: VisibleSignature = {
  enabled: true,
  rubric: false,
  fields: { signerName: true, issuer: false, signedAt: true, reason: false },
  reason: "",
};
