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
  compose(signature: VisibleSignature, signer: SigningIdentity): Promise<string | null>;
}

/**
 * Con qué y cuándo se firma, que es lo que le faltaba al compositor.
 *
 * El recuadro lleva el nombre y el DNI del titular —que solo conoce quien lee
 * el DER, o sea el backend— y la fecha. Por el puerto viajan el **asa** del
 * certificado, con la que el backend lo reencuentra, y el **instante ya
 * formateado**.
 *
 * `signedAt` tiene que ser **el mismo** que se envíe a firmar. El recuadro se
 * compone antes de la prefirma y el PDF ya no se vuelve a tocar, así que
 * componer la vista previa con una hora y firmar con otra sería enseñar algo
 * que el PDF no va a tener. Por eso se fija una vez y se pasa a las dos.
 */
export interface SigningIdentity {
  /** El asa del certificado elegido, la que dio el backend al listar. */
  certificate: string;
  /** La fecha y hora, ya formateadas para el recuadro. */
  signedAt: string;
  /** El idioma en el que van las etiquetas del recuadro. */
  language: string;
}

/**
 * Un compositor que no compone nada: la vista previa se queda en su estado
 * vacío.
 *
 * Desde el #60 el autoritativo es `tauriLayer2Composer`, que pide el texto a
 * `signing::layer2_text`; esto queda como doble de pruebas, igual que
 * `emptyPdfSource`.
 */
export function emptyLayer2Composer(): Layer2Composer {
  return { compose: async () => null };
}
