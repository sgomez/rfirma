/**
 * Las tres etapas de la firma trifásica, en el lado de la interfaz.
 *
 * La clave privada **nunca** sale de la tarjeta: Java prepara (prefirma) y
 * ensambla (postfirma), y la única etapa que toca la clave es la de en medio,
 * que corre en Rust contra el módulo PKCS#11. La interfaz no firma nada; solo
 * pide cada etapa por su turno y enseña en cuál va.
 */

import type { TokenFailure } from "./token";
import type { VisibleTextFields } from "./visibleSignature";

/** Las tres etapas, en el orden en que ocurren. */
export type SigningStage = "presign" | "sign" | "postsign";

/** El orden es dato, no adorno: el diálogo de progreso lo recorre. */
export const SIGNING_STAGES: readonly SigningStage[] = ["presign", "sign", "postsign"];

/** El documento ya firmado, tal como lo deja el destino (ADR-0011). */
export interface SignedDocument {
  /** El nombre del fichero resultante. La ruta no se enseña nunca. */
  name: string;
  /** El nombre de la carpeta donde quedó, no su ruta. */
  folder: string;
}

/**
 * La orden de firma: todo lo que distingue esta firma de otra.
 *
 * Va entera en la etapa 1 y **no se guarda en la ventana** a medias: entre la
 * prefirma y la postfirma el ciclo vive en el backend, que es quien tiene el
 * sello de sesión. Lo que la ventana no tiene no lo puede alterar (ADR-0016).
 */
export interface SigningOrder {
  /** El identificador que acuñó el backend al abrir el documento (ID-62). */
  document: string;
  /** El `CKA_LABEL` del certificado elegido. */
  certificate: string;
  /**
   * Dónde cae el recuadro, en **espacio de usuario PDF** (ID-21), con la
   * `MediaBox` y la `/Rotate` de la página.
   *
   * No en puntos PAdES: la inversa de la rotación que iText aplica al cerrar el
   * documento la hace `signing::placement` en el backend, y con ella viene la
   * guardia del ID-22. Una tabla por rotación en TypeScript sería una copia de
   * ese módulo, y divergiría en la primera esquina.
   */
  placement: {
    /** Página **1-based**, como la numera `pdf.js`. */
    page: number;
    /** `[x0, y0, x1, y1]` de la `MediaBox`. */
    mediaBox: readonly [number, number, number, number];
    /** La `/Rotate` en grados. */
    rotation: number;
    /** `[x0, y0, x1, y1]` del recuadro en espacio de usuario. */
    rect: readonly [number, number, number, number];
  };
  /** Qué casillas van dentro del recuadro. */
  fields: VisibleTextFields;
  /** El motivo. Vacío es «sin motivo». */
  reason: string;
  /** La fecha y hora ya formateadas, **las mismas** de la vista previa. */
  signedAt: string;
  /** La rúbrica ya normalizada, en Base64; `null` si no la hay. */
  rubric: string | null;
  /** El idioma de las etiquetas del recuadro. */
  language: string;
}

/** Lo que devuelve una etapa: salió, o falló con una situación clasificada. */
export type StageResult<T> = { ok: true; value: T } | { ok: false; failure: TokenFailure };

/**
 * Quien firma de verdad. Puerto por lo mismo que los demás: el recorrido vive
 * en la ventana y las tres etapas viven en el backend.
 *
 * Las tres son métodos distintos y no una sola llamada **a propósito**: entre
 * la prefirma y la firma se pide el PIN, y el diálogo de progreso enseña en qué
 * etapa va. Una única `sign()` que hiciera las tres dejaría a la interfaz sin
 * nada que contar durante los segundos que tarda la postfirma.
 */
export interface SigningBackend {
  /** Etapa 1: prepara lo que hay que firmar. No toca la clave privada. */
  presign(order: SigningOrder): Promise<StageResult<void>>;
  /** Etapa 2: firma el hash en la tarjeta, con el PIN que el usuario tecleó. */
  sign(pin: string): Promise<StageResult<void>>;
  /** Etapa 3: ensambla el PDF firmado y lo deja en el destino. */
  postsign(): Promise<StageResult<SignedDocument>>;
  /**
   * Olvida el ciclo a medias: la cuarta operación **no es una etapa**, es la
   * salida.
   *
   * Está en el puerto y no suelta en `tauri.ts` porque el ciclo a medias lo
   * guarda el backend, y quien lo abre es quien tiene que poder cerrarlo. Sin
   * esto, cancelar en el diálogo del PIN devolvía la ventana al panel y dejaba
   * vivos —hasta cerrar la aplicación— el PDF en Base64, los atributos CAdES a
   * firmar, el sello de sesión y, si ya se había tecleado, el PKCS#1 de la
   * tarjeta.
   *
   * Es idempotente: cancelar sin ciclo abierto no es un fallo.
   */
  discard(): Promise<void>;
}

/**
 * Un firmante que **no firma**, y lo dice.
 *
 * Desde el #60 quien firma de verdad es `tauriSigningBackend`, así que esto ya
 * no es el relleno de `main.tsx` sino un doble: sirve para montar la ventana en
 * una prueba sin backend, y para cualquier composición que no deba llegar a
 * firmar. Falla diciendo la verdad en vez de fingir una firma, que es lo único
 * que no puede hacer un doble en una aplicación de firma.
 */
export function unavailableSigningBackend(): SigningBackend {
  const missing = <T>(): Promise<StageResult<T>> =>
    Promise.resolve({
      ok: false,
      failure: {
        situation: "unknown",
        detail: "no hay orden de firma expuesta todavia",
        attemptsLeft: null,
      },
    });
  return { presign: missing, sign: missing, postsign: missing, discard: async () => {} };
}
