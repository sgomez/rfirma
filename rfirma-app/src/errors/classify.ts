import type { ErrorSituation } from "./ErrorNotice";

/**
 * Un fallo con la forma del ID-29: una **situación** nuestra, que el catálogo
 * traduce, y el texto original **crudo** al lado.
 *
 * Es la misma forma que `commands::Failure` en Rust, campo a campo.
 */
export interface NamedFailure {
  situation: ErrorSituation;
  /** El texto original, sin traducir ni recortar. Nunca vacío. */
  detail: string;
  /** Cuántos intentos de PIN quedan, cuando el módulo lo dice. */
  attemptsLeft: number | null;
}

/** Lo que rechaza una orden cuando el fallo viene ya clasificado por Rust. */
interface RejectedFailure {
  situation: string;
  detail: string;
  attemptsLeft?: number | null;
}

function isRejectedFailure(thrown: unknown): thrown is RejectedFailure {
  return (
    typeof thrown === "object" &&
    thrown !== null &&
    typeof (thrown as RejectedFailure).situation === "string" &&
    typeof (thrown as RejectedFailure).detail === "string"
  );
}

/**
 * Clasifica lo que sea que haya rechazado.
 *
 * Un rechazo con forma se pasa tal cual —la situación ya viene clasificada por
 * Rust—; cualquier otra cosa cae en `unknown` **conservando su texto**. Perder
 * ese texto sería quedarse sin lo único que sirve para diagnosticar el fallo
 * que no supimos clasificar (ADR-0009).
 *
 * Vive aquí y no en `tauri.ts` porque no es conocimiento de Tauri: es la forma
 * del ID-29, y la comparten el puente de firma y el visor.
 */
export function classify(thrown: unknown): NamedFailure {
  if (isRejectedFailure(thrown)) {
    return {
      situation: thrown.situation as ErrorSituation,
      detail: thrown.detail,
      attemptsLeft: thrown.attemptsLeft ?? null,
    };
  }
  return {
    situation: "unknown",
    detail: thrown instanceof Error ? thrown.message : String(thrown),
    attemptsLeft: null,
  };
}
