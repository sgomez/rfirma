/** Clasifica el fallo de una orden de Tauri y envuelve su llamada. Lo comparten `tauriSigning.ts` y `tauriSede.ts`. */

import { classify } from "./errors/classify";
import type { StageResult } from "./signing/flow";
import type { TokenFailure } from "./signing/token";

function failureOf(thrown: unknown): TokenFailure {
  const named = classify(thrown);
  return {
    situation: named.situation as TokenFailure["situation"],
    detail: named.detail,
    attemptsLeft: named.attemptsLeft,
  };
}

/**
 * Envuelve una etapa: sale bien, o sale con una situación clasificada.
 *
 * Recibe **la llamada sin hacer** y no la promesa ya hecha, para que la orden
 * se invoque **dentro** del `try`. Con la promesa por parámetro, un fallo
 * síncrono de `invoke` —una orden que no existe— se escaparía de este `catch`,
 * y el rechazo quedaría suelto entre que se crea y que se espera.
 */
export async function stage<T>(call: () => Promise<T>): Promise<StageResult<T>> {
  try {
    return { ok: true, value: await call() };
  } catch (thrown) {
    return { ok: false, failure: failureOf(thrown) };
  }
}
