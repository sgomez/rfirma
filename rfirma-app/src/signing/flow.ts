/**
 * Las tres etapas de la firma trifásica, en el lado de la interfaz.
 *
 * La clave privada **nunca** sale de la tarjeta: Java prepara (prefirma) y
 * ensambla (postfirma), y la única etapa que toca la clave es la de en medio,
 * que corre en Rust contra el módulo PKCS#11. La interfaz no firma nada; solo
 * pide cada etapa por su turno y enseña en cuál va.
 */

import type { TokenFailure } from "./token";

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
  presign(): Promise<StageResult<void>>;
  /** Etapa 2: firma el hash en la tarjeta, con el PIN que el usuario tecleó. */
  sign(pin: string): Promise<StageResult<void>>;
  /** Etapa 3: ensambla el PDF firmado y lo deja en el destino. */
  postsign(): Promise<StageResult<SignedDocument>>;
}

/**
 * El que firma mientras no hay órdenes expuestas que lo hagan (#60).
 *
 * No es alcanzable hoy: sin almacén de certificados no hay certificado en
 * vigor, y sin él el botón de firmar está apagado. Existe para que `main.tsx`
 * pueda enchufar los puertos como los demás, y falla diciendo la verdad en vez
 * de fingir una firma.
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
  return { presign: missing, sign: missing, postsign: missing };
}
