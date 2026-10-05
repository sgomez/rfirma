//! La conversión pura del momento del backend al `Errand` de la ventana, y del fallo de una etapa al desenlace de rechazo que nombra la tabla de rechazos (`REFUSAL_ACTION_OF`). Sin React.

import type { PreviousSignaturesReport } from "../signing/previousSignatures";
import {
  type Errand,
  type ErrandStage,
  isSedeRefusal,
  NAMED_BY_THE_DESK,
  type NamedByTheDesk,
  type RefusalSituation,
  type SignatureRound,
  type SiteDocument,
  type SiteOutcome,
} from "./errand";
import type { DescribedDocument, SiteErrandView, SiteStageView } from "./siteErrandView";

/** Las etiquetas del backend que la tabla de rechazos ya nombra de otro modo: las del lote, sin su prefijo. */
const RENAMED: Record<string, RefusalSituation> = {
  unreadable: "cannotLoadData",
  saveDestinationUnwritable: "cannotSaveData",
  presignerUnreachable: "batchPresignerUnreachable",
  postsignerUnreachable: "batchPostsignerUnreachable",
  invalidPresignResponse: "batchInvalidPresignResponse",
  invalidPostsignResponse: "batchInvalidPostsignResponse",
};

/** La situación tal como la nombra la tabla de rechazos, o `unknown`. */
function refusalOf(situation: string): RefusalSituation {
  const renamed = RENAMED[situation];
  if (renamed !== undefined) return renamed;
  if (isSedeRefusal(situation) || isNamedByTheDesk(situation)) return situation;
  return "unknown";
}

function isNamedByTheDesk(situation: string): situation is NamedByTheDesk {
  return (NAMED_BY_THE_DESK as readonly string[]).includes(situation);
}

/** Un fallo de una etapa, contado como el desenlace que la ventana enseña. */
export function refusedBy(failure: { situation: string; detail: string }): SiteOutcome {
  return { kind: "refused", situation: refusalOf(failure.situation), detail: failure.detail };
}

/**
 * Lo mismo, sabiendo que lo que falló era un lote: la situación del token
 * (`incorrectPin`, `tokenAbsent`…) se cuenta con su título, y la que no tiene
 * texto propio, como «lote fallido».
 */
export function refusedByTheBatch(failure: { situation: string; detail: string }): SiteOutcome {
  const named = refusalOf(failure.situation);
  return {
    kind: "refused",
    situation: named === "unknown" ? "batchSigningFailed" : named,
    detail: failure.detail,
  };
}

/**
 * **El momento del backend, en el vocabulario de la ventana**.
 *
 * La operación no viaja en el evento porque está en el momento: la sede que
 * sólo pide identidad manda `askingForConsent`, y la que manda un documento
 * manda `askingToSign`. En la espera todavía no se sabe cuál de las dos es, y
 * ahí `operation` no la mira nadie —`consentActionKey` sólo se consulta al
 * consentir—.
 *
 * `document` llega aparte porque leerlo por su asa es una ida y vuelta al
 * backend, y esta función es pura.
 */
export function errandOf(view: SiteErrandView, document: SiteDocument | null = null): Errand {
  const stage = view.stage;
  const operation = stage.kind === "askingForConsent" ? "selectcert" : "sign";
  return { origin: view.origin, operation, stage: stageOf(stage, document) };
}

function stageOf(stage: SiteStageView, document: SiteDocument | null): ErrandStage {
  switch (stage.kind) {
    case "waiting":
      return { kind: "waiting" };
    case "unreachable":
      return { kind: "unreachable" };
    case "oldWebClient":
      return { kind: "oldWebClient" };
    case "noChannel":
      return { kind: "noChannel", reason: stage.reason };
    case "noCertificate":
      return { kind: "noCertificate", reason: stage.reason, owned: stage.owned };
    case "saving":
      return { kind: "saving", filename: stage.filename };
    case "loading":
      return { kind: "loading", multiple: stage.multiple };
    case "outcome":
      return {
        kind: "outcome",
        outcome: {
          kind: "refused",
          situation: refusalOf(stage.outcome.situation),
          detail: stage.outcome.detail,
        },
      };
    case "askingForConsent":
      // Sin documento porque no lo hay: `selectcert` no manda ninguno. Y
      // `narrowed` es `false` porque el backend no dice si la sede acotó la
      // lista: lo que cruza son las filas ya cribadas y nunca el criterio.
      return {
        kind: "consent",
        document: null,
        signs: null,
        signing: null,
        items: null,
        certificates: stage.certificates,
        narrowed: false,
        sha1Allowed: false,
      };
    case "askingToSign":
      return {
        kind: "consent",
        document,
        signs: null,
        signing: stage.signing,
        items: null,
        certificates: stage.certificates,
        narrowed: false,
        sha1Allowed: stage.sha1Allowed,
      };
    case "askingToConfirm":
      return { kind: "confirming", messageCode: stage.messageCode };
    case "markingTheArea":
      return { kind: "marking", pdf: null };
    case "askingToSignTheBatch":
      // Sin documento porque el lote no manda ninguno: sus ficheros se quedan
      // en la sede y lo que se consiente es cuántas firmas van a salir.
      return {
        kind: "consent",
        document: null,
        signs: stage.signs,
        signing: null,
        items: null,
        certificates: stage.certificates,
        narrowed: false,
        sha1Allowed: false,
      };
    case "askingToSignTheLocalBatch":
      // Igual que el lote remoto, y además con el resumen de cada elemento:
      // sin él, un lote podría colar un documento que nadie consintió.
      return {
        kind: "consent",
        document: null,
        signs: stage.items.length,
        signing: null,
        items: stage.items,
        certificates: stage.certificates,
        narrowed: false,
        sha1Allowed: false,
      };
  }
}

/**
 * El documento del consentimiento, con lo que dice de sí mismo y lo que la
 * sede pide firmar sobre lo que ya trae.
 *
 * La ronda cruza entera y sin recuento: cuántas firmas lleva el PDF no lo
 * cuenta nadie, y lo que la ficha pide enseñar es qué será la firma de la
 * persona —cofirma, o contrafirma sobre todas o sobre las últimas—.
 */
export function documentOf(
  described: DescribedDocument | null,
  round: SignatureRound,
  previousSignatures: PreviousSignaturesReport,
): SiteDocument | null {
  if (described === null) return null;
  return {
    ...described,
    round,
    previousSignatures,
  };
}

/** Qué documento se estaba consintiendo, para poder nombrarlo en el desenlace. */
export function documentInPlay(errand: Errand | null): SiteDocument | null {
  const stage = errand?.stage;
  return stage?.kind === "consent" ? stage.document : null;
}
