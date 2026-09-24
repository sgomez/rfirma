import type { Catalog } from "../i18n/catalog";
import {
  type Errand,
  type ErrandStage,
  NAMED_BY_THE_DESK,
  type RefusalSituation,
  type SignatureRound,
  type SiteDocument,
  type SiteOutcome,
} from "./errand";
import type { DescribedDocument, SiteErrandView, SiteStageView } from "./siteErrandView";

/**
 * Las situaciones de rechazo que el catálogo sabe redactar.
 *
 * Un `Record` y no una lista: si `sede.refusals` gana una clave, `tsc` exige
 * que entre también aquí, y ninguna situación nueva acaba cayendo en `unknown`
 * sin que nadie se entere.
 */
const REFUSALS: Record<keyof Catalog["sede"]["refusals"], true> = {
  appendedSignaturePage: true,
  unsupportedFilter: true,
  unsupportedProtocolVersion: true,
  missingFormat: true,
  unsupportedKeyStore: true,
  errandInFlight: true,
  portsTaken: true,
  saveCancelled: true,
  loadCancelled: true,
  cannotSaveData: true,
  cannotLoadData: true,
  batchPresignerUnreachable: true,
  batchPostsignerUnreachable: true,
  batchInvalidPresignResponse: true,
  batchInvalidPostsignResponse: true,
  batchSigningFailed: true,
  triphaseServerUrlMissing: true,
  triphaseServerException: true,
  triphaseServerUnreachable: true,
  triphaseServerUnexpectedAnswer: true,
  certificateNotFound: true,
  folderMissing: true,
  unwritable: true,
  invalidSignature: true,
  confirmationNeeded: true,
  localBatchSign: true,
  siteErrandNotLive: true,
  pdfHasUnregisteredSignatures: true,
  secretOnTheReaderKeypad: true,
  userCancelled: true,
  promptFailed: true,
  unknown: true,
};

/** Las etiquetas del backend que el catálogo ya redacta con otro nombre: las del lote, sin su prefijo. */
const RENAMED: Record<string, RefusalSituation> = {
  unreadable: "cannotLoadData",
  presignerUnreachable: "batchPresignerUnreachable",
  postsignerUnreachable: "batchPostsignerUnreachable",
  invalidPresignResponse: "batchInvalidPresignResponse",
  invalidPostsignResponse: "batchInvalidPostsignResponse",
};

/** La situación tal como la sabe nombrar el catálogo, o `unknown`. */
function refusalOf(situation: string): RefusalSituation {
  const renamed = RENAMED[situation];
  if (renamed !== undefined) return renamed;
  if (situation in REFUSALS || isNamedByTheDesk(situation)) return situation as RefusalSituation;
  return "unknown";
}

function isNamedByTheDesk(situation: string): boolean {
  return (NAMED_BY_THE_DESK as readonly string[]).includes(situation);
}

/** Un fallo de una etapa, contado como el desenlace que la ventana enseña. */
export function refusedBy(failure: { situation: string; detail: string }): SiteOutcome {
  return { kind: "refused", situation: refusalOf(failure.situation), detail: failure.detail };
}

/**
 * Lo mismo, sabiendo que lo que falló era un lote: sus fallos de firma llegan
 * con la situación del token (`incorrectPin`, `tokenAbsent`…), y el lote los
 * llama «lote fallido».
 */
export function refusedByTheBatch(failure: { situation: string; detail: string }): SiteOutcome {
  const named = refusalOf(failure.situation);
  return {
    kind: "refused",
    situation: named === "unknown" || isNamedByTheDesk(named) ? "batchSigningFailed" : named,
    detail: failure.detail,
  };
}

/**
 * **El momento del backend, en el vocabulario de la ventana** (TD-78).
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
      // lista: lo que cruza son las filas ya cribadas y nunca el criterio
      // (ID-277).
      return {
        kind: "consent",
        document: null,
        signs: null,
        signing: null,
        items: null,
        certificates: stage.certificates,
        narrowed: false,
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
  unregisteredSignatures: boolean,
): SiteDocument | null {
  if (described === null) return null;
  return { ...described, round, hasUnregisteredSignatures: unregisteredSignatures };
}

/** Qué documento se estaba consintiendo, para poder nombrarlo en el desenlace. */
export function documentInPlay(errand: Errand | null): SiteDocument | null {
  const stage = errand?.stage;
  return stage?.kind === "consent" ? stage.document : null;
}
