/**
 * Qué se estampa en el recuadro de la firma visible.
 *
 * **Casillas, no comodines** (ID-19). El usuario nunca escribe `$$SUBJECTCN$$`
 * ni `$$SIGNDATE$$`: marca qué dato aparece, y el texto lo compone Rust en
 * `signing::layer2_text` con las etiquetas y la máscara del DNI de AutoFirma.
 * Aquí no hay ni una cadena del recuadro: si te encuentras escribiendo
 * «Firmado por» o una máscara de asteriscos en este directorio, estás
 * duplicando ese módulo, que es exactamente lo que la vista previa evita
 * pidiéndole el texto ya compuesto (ver [`Layer2Composer`]).
 */

/** Las cuatro casillas de texto. La rúbrica va aparte: es una imagen. */
export interface VisibleTextFields {
  signerName: boolean;
  idNumber: boolean;
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
 * Lo que sale marcado la primera vez: recuadro sí, y dentro el nombre, el DNI y
 * la fecha, que es el contenido de un recuadro administrativo corriente. La
 * rúbrica no, porque todavía no hay imagen; el motivo tampoco, porque está
 * vacío y una etiqueta «Motivo:» sin nada detrás no dice nada.
 */
export const DEFAULT_VISIBLE_SIGNATURE: VisibleSignature = {
  enabled: true,
  rubric: false,
  fields: { signerName: true, idNumber: true, signedAt: true, reason: false },
  reason: "",
};

/**
 * El texto del recuadro, ya compuesto, para enseñarlo antes de firmar.
 *
 * Es un puerto y no una función local **a propósito**: el compositor
 * autoritativo es `signing::layer2_text` en Rust —etiquetas por idioma, máscara
 * del DNI replicada de `PdfVisibleAreasUtils` con sus tres rarezas— y la vista
 * previa solo es honesta si enseña esa misma cadena. Una copia en TypeScript
 * empezaría igual y divergiría en la primera esquina.
 */
export interface Layer2Composer {
  /** El texto tal cual irá en `layer2Text`, o `null` si aún no hay backend. */
  compose(signature: VisibleSignature): Promise<string | null>;
}

/**
 * El compositor mientras no hay orden expuesta que lo pida: no compone nada y
 * la vista previa se queda en su estado vacío. Doble de pruebas y relleno de
 * `main.tsx`, igual que `emptyPdfSource`.
 */
export function emptyLayer2Composer(): Layer2Composer {
  return { compose: async () => null };
}
