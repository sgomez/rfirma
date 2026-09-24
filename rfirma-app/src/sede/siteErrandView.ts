import type { Certificate } from "../signing/certificate";
import type { LocalBatchItem, SignatureRound, SigningKind } from "./errand";

/**
 * **El trámite tal como lo emite el backend**: `commands::SiteErrandView`,
 * campo a campo.
 *
 * Detrás no hay ninguna ruta (ADR-0011): el documento que manda la sede viaja
 * por su **asa opaca** y el origen viaja **a secas**, sólo para atribuir
 * (ID-271, ID-339).
 */
export interface SiteErrandView {
  origin: string | null;
  stage: SiteStageView;
}

/** El momento de la secuencia, tal como lo emite el backend. */
export type SiteStageView =
  | { kind: "waiting" }
  | { kind: "askingForConsent"; certificates: readonly Certificate[] }
  /** La sede pide `visibleSignature`: el área se marca sobre el PDF antes del consentimiento. */
  | { kind: "markingTheArea"; document: string }
  | {
      kind: "askingToSign";
      /** El asa opaca con la que se lee el documento, nunca su ruta (ID-286). */
      document: string;
      /** Qué es lo que se pide firmar, según el formato de la petición (#530). */
      signing: SigningKind;
      round: SignatureRound;
      certificates: readonly Certificate[];
      unregisteredSignatures: boolean;
      /** El asa que `headless` ya resolvió: la única fila que pasó el filtro. */
      alreadyChosen: string | null;
    }
  | {
      kind: "askingToConfirm";
      /** El mensaje con el que pregunta el original, por su código. */
      messageCode: string;
    }
  | {
      kind: "askingToSignTheBatch";
      /** Cuántas firmas lleva el lote. */
      signs: number;
      certificates: readonly Certificate[];
      /** El asa que `sticky` preselecciona: la fijada en la sesión de sede, la que el desplegable elige sola. */
      alreadyChosen: string | null;
    }
  | {
      kind: "askingToSignTheLocalBatch";
      /** Los elementos del lote, en el orden en que la sede los declaró. */
      items: readonly LocalBatchItem[];
      certificates: readonly Certificate[];
      /** El asa que `sticky` preselecciona: la fijada en la sesión de sede, la que el desplegable elige sola. */
      alreadyChosen: string | null;
    }
  | { kind: "saving"; filename: string | null }
  | { kind: "loading"; multiple: boolean }
  | { kind: "noChannel"; reason: "channelNotOpened" | "localCaMissing" }
  | { kind: "outcome"; outcome: { kind: "refused"; situation: string; detail: string } }
  | { kind: "noCertificate"; reason: "none" | "excluded"; owned: number }
  | { kind: "unreachable" }
  | { kind: "oldWebClient" };

/**
 * Lo que el PDF de la sede dice de sí mismo, leído por su asa.
 *
 * Es lo único que se puede enseñar del documento: la petición **no trae
 * nombre** (ID-270) y de la ruta del fichero de paso no llega nada. Sale de
 * abrir los bytes que devuelve `read_document`, así que `null` es que no se han
 * podido leer, y entonces no hay tarjeta que pintar.
 */
export interface DescribedDocument {
  title: string | null;
  pages: number;
  sizeBytes: number;
}

/**
 * Cómo acaba una orden del portal: `TokenFailure` clasifica situaciones de
 * PKCS#11 y no le sirve al diálogo del portal, que rechaza con las suyas
 * propias (`saveCancelled`, `cannotLoadData`…), así que aquí el rechazo va sin
 * clasificar y `refusalOf` lo traduce al mismo catálogo que el resto.
 */
export type PortalResult<T> =
  | { ok: true; value: T }
  | { ok: false; failure: { situation: string; detail: string } };

/** El rechazo de una orden tal como cruza: la situación sin clasificar y el detalle crudo. */
interface UnclassifiedFailure {
  situation: string;
  detail: string;
  attemptsLeft: number | null;
}

/** Cómo acaba la orden del secreto, que en el lote firma y entrega de una vez. */
export type SecretResult<T> = { ok: true; value: T } | { ok: false; failure: UnclassifiedFailure };
