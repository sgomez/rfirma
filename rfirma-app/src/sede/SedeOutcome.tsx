import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import {
  AlertIcon,
  CheckCircleIcon,
  CopyIcon,
  CrossCircleIcon,
  ExternalLinkIcon,
  FileIcon,
} from "../design-system/icons";
import type { ExternalDestinationOpener } from "../desktop/externalDestination";
import { formatSize } from "../signing/SigningPanel";
import {
  OUTCOME_CLOSE_MS,
  type RefusalSituation,
  type SiteDocument,
  type SiteOutcome,
} from "./errand";
import { SedeBody, useOutcomeClock } from "./SedeFrame";

interface SedeOutcomeProps {
  origin: string | null;
  outcome: SiteOutcome;
  onClose: () => void;
  onOpenHelp?: () => void;
  externalDestinations?: ExternalDestinationOpener;
}

/**
 * **4 · Desenlace.** En todos ellos **la sede ya ha recibido su respuesta**: los
 * dos canales van desacompasados a propósito (#316). Salvo el rechazo de la
 * petición misma, que sale al cerrar, como el diálogo de error del original.
 *
 * El **rechazo** cubre los del transporte, que ocurren antes de que haya nada
 * que consentir. El argumento para enseñarlo no es que la persona pueda
 * arreglarlo —no puede—: es que acaba de arrancarse un programa en su equipo a
 * petición de una web, y un rFirma que aparece y desaparece en silencio es
 * indistinguible de uno roto. Lo único accionable es el detalle copiable.
 *
 * **Se cierra sola a los quince segundos**, no a los cinco: con cinco no daba
 * tiempo a leer, y el caso que lo decide es el rechazo, donde irse sola
 * reproduciría el síntoma que el aviso venía a evitar (ID-274).
 */
export function SedeOutcome({
  origin,
  outcome,
  onClose,
  onOpenHelp,
  externalDestinations,
}: SedeOutcomeProps) {
  const { t } = useTranslation();
  const carriesHelp = outcome.kind === "refused" && outcome.situation === "unknown";
  const asksToAct = outcome.kind === "refused" && outcome.situation === "portsTaken";
  const staysOpen = carriesHelp || asksToAct;
  useOutcomeClock(onClose, !staysOpen);

  const openHelp = () => {
    onOpenHelp?.();
    void externalDestinations?.open("discussions");
  };

  return (
    <SedeBody
      steadyFooter
      footer={
        <>
          {!staysOpen && (
            <p className="rf-hint sede-outcome__auto-close">
              {t("sede.outcome.autoClose", { seconds: OUTCOME_CLOSE_MS / 1000 })}
            </p>
          )}
          <div className="sede-window__spacer" />
          <button type="button" className="rf-btn rf-btn--primary" onClick={onClose}>
            {t("actions.close")}
          </button>
        </>
      }
    >
      <div className="rf-stack sede-outcome">
        <div className="rf-row rf-gap-xs sede-outcome__head">
          <span className="sede-outcome__icon">
            <OutcomeIcon kind={outcome.kind} />
          </span>
          <p className="rf-title sede-outcome__title">{title(outcome, t)}</p>
        </div>
        {(outcome.kind === "signed" || outcome.kind === "cancelled") &&
          outcome.document !== null && (
            /* Lo único que dice **qué** se acaba de firmar —o dejar sin firmar—
             en la pantalla que confirma que rFirma no guarda copia. En el
             rechazo no se enseña porque ahí nunca llegó a haber documento. */
            <DocumentRow document={outcome.document} />
          )}
        {outcome.kind === "signed" && (
          <>
            <p className="rf-prose">
              {origin === null
                ? t("sede.outcome.signedBodyUnknownOrigin")
                : t("sede.outcome.signedBody", { origin })}
            </p>
            {/* La única de las tres frases que no se deduce mirando: la
                aplicación **sí** tiene bandeja de recientes, y aquí no entra
                nada. */}
            <p className="rf-hint">{t("sede.outcome.signedNote")}</p>
          </>
        )}
        {outcome.kind === "batchSigned" && (
          <>
            <p className="rf-prose">
              {origin === null
                ? t("sede.outcome.batchBodyUnknownOrigin", { count: outcome.signs })
                : t("sede.outcome.batchBody", { count: outcome.signs, origin })}
            </p>
            {/* La misma tranquilidad que tras firmar un documento, y aquí con
                más motivo: los ficheros del lote nunca salieron de la sede. */}
            <p className="rf-hint">{t("sede.outcome.signedNote")}</p>
          </>
        )}
        {outcome.kind === "saved" && (
          <p className="rf-prose">
            {origin === null
              ? t("sede.outcome.savedBodyUnknownOrigin")
              : t("sede.outcome.savedBody", { origin })}
          </p>
        )}
        {outcome.kind === "loaded" && (
          <p className="rf-prose">
            {origin === null
              ? t("sede.outcome.loadedBodyUnknownOrigin", { count: outcome.fileCount })
              : t("sede.outcome.loadedBody", { count: outcome.fileCount, origin })}
          </p>
        )}
        {outcome.kind === "refused" && (
          <>
            <p className="rf-prose">
              <RefusalSentence situation={outcome.situation} origin={origin} />
            </p>
            <p className="rf-hint">{t("sede.outcome.refusedNote")}</p>
            <SiteNote situation={outcome.situation} />
            <div className="rf-stack rf-gap-xs sede-outcome__detail">
              <div className="rf-row rf-gap-xs sede-outcome__detail-head">
                <span className="rf-label">{t("sede.outcome.detail")}</span>
                <button
                  type="button"
                  className="rf-btn rf-btn--ghost sede-outcome__copy"
                  onClick={() => void navigator.clipboard.writeText(outcome.detail)}
                >
                  <CopyIcon size={14} />
                  {t("actions.copy")}
                </button>
              </div>
              <code className="rf-body sede-outcome__detail-text">{outcome.detail}</code>
            </div>
            {outcome.situation === "unknown" && (
              <div className="rf-row rf-gap-xs sede-outcome__report">
                <p className="rf-hint">{t("sede.outcome.reportHint")}</p>
                <button
                  type="button"
                  className="rf-btn rf-btn--ghost sede-outcome__help"
                  onClick={openHelp}
                >
                  <ExternalLinkIcon size={14} />
                  {t("errors.help")}
                </button>
              </div>
            )}
          </>
        )}
      </div>
    </SedeBody>
  );
}

/** Los tres del artboard: visto, aspa y triángulo, uno por desenlace. */
function OutcomeIcon({ kind }: { kind: SiteOutcome["kind"] }) {
  switch (kind) {
    case "signed":
    case "batchSigned":
    case "saved":
    case "loaded":
      return <CheckCircleIcon size={24} />;
    case "cancelled":
      return <CrossCircleIcon size={24} />;
    default:
      return <AlertIcon size={24} />;
  }
}

/**
 * El documento en una línea: la misma información que se enseñó al consentir,
 * sin la cofirma —ya no hay nada que decidir— y sin nombre de fichero, que el
 * protocolo no trae.
 */
function DocumentRow({ document }: { document: SiteDocument }) {
  const { t, i18n } = useTranslation();
  const untitled = document.title === null || document.title.trim() === "";

  return (
    <div className="rf-row rf-gap-xs sede-outcome__document">
      <span className="sede-outcome__icon">
        <FileIcon size={18} />
      </span>
      <p className={`rf-prose sede-outcome__document-title${untitled ? " rf-text-muted" : ""}`}>
        {untitled ? t("sede.consent.untitled") : document.title}
      </p>
      <span className="rf-body rf-text-muted sede-outcome__document-meta">
        {[
          t("panel.document.pages", { count: document.pages }),
          formatSize(document.sizeBytes, i18n.language),
        ].join(" · ")}
      </span>
    </div>
  );
}

/**
 * La incompatibilidad, enunciada nombrando el origen y **sin acusar a nadie**:
 * se dice el hecho y quien lee saca la conclusión.
 *
 * Cada clave se escribe **entera**, sin plantilla: una clave
 * ensamblada con plantilla no la ve ni `extract --ci` ni `status --unused`
 * (`src/AGENTS.md`). Lo que ya nombra el escritorio se cuenta con su título.
 *
 * Sin origen válido el sujeto es la petición: nombrar a secas atribuye sin
 * afirmar, y el hueco tampoco se rellena con un invento.
 */
function RefusalSentence({
  situation,
  origin,
}: {
  situation: RefusalSituation;
  origin: string | null;
}) {
  const { t } = useTranslation();
  const subject = { origin: origin ?? t("sede.origin.unknown") };

  switch (situation) {
    case "appendedSignaturePage":
      return <>{t("sede.refusals.appendedSignaturePage", subject)}</>;
    case "unsupportedFilter":
      return <>{t("sede.refusals.unsupportedFilter", subject)}</>;
    case "unsupportedProtocolVersion":
      return <>{t("sede.refusals.unsupportedProtocolVersion", subject)}</>;
    case "missingFormat":
      return <>{t("sede.refusals.missingFormat", subject)}</>;
    case "unsupportedKeyStore":
      return <>{t("sede.refusals.unsupportedKeyStore", subject)}</>;
    case "errandInFlight":
      return <>{t("sede.refusals.errandInFlight", subject)}</>;
    case "portsTaken":
      return <>{t("sede.refusals.portsTaken", subject)}</>;
    case "explicitXades":
      return <>{t("sede.refusals.explicitXades", subject)}</>;
    case "invoiceMultisignature":
      return <>{t("sede.refusals.invoiceMultisignature", subject)}</>;
    case "unsupportedCountersignature":
      return <>{t("sede.refusals.unsupportedCountersignature", subject)}</>;
    case "saveCancelled":
      return <>{t("sede.refusals.saveCancelled", subject)}</>;
    case "loadCancelled":
      return <>{t("sede.refusals.loadCancelled", subject)}</>;
    case "cannotSaveData":
      return <>{t("sede.refusals.cannotSaveData", subject)}</>;
    case "cannotLoadData":
      return <>{t("sede.refusals.cannotLoadData", subject)}</>;
    case "batchPresignerUnreachable":
      return <>{t("sede.refusals.batchPresignerUnreachable", subject)}</>;
    case "batchPostsignerUnreachable":
      return <>{t("sede.refusals.batchPostsignerUnreachable", subject)}</>;
    case "batchInvalidPresignResponse":
      return <>{t("sede.refusals.batchInvalidPresignResponse", subject)}</>;
    case "batchInvalidPostsignResponse":
      return <>{t("sede.refusals.batchInvalidPostsignResponse", subject)}</>;
    case "batchSigningFailed":
      return <>{t("sede.refusals.batchSigningFailed", subject)}</>;
    case "triphaseServerUrlMissing":
      return <>{t("sede.refusals.triphaseServerUrlMissing", subject)}</>;
    case "triphaseServerException":
      return <>{t("sede.refusals.triphaseServerException", subject)}</>;
    case "triphaseServerUnreachable":
      return <>{t("sede.refusals.triphaseServerUnreachable", subject)}</>;
    case "triphaseServerUnexpectedAnswer":
      return <>{t("sede.refusals.triphaseServerUnexpectedAnswer", subject)}</>;
    case "certificateNotFound":
      return <>{t("sede.refusals.certificateNotFound", subject)}</>;
    case "folderMissing":
      return <>{t("sede.refusals.folderMissing", subject)}</>;
    case "unwritable":
      return <>{t("sede.refusals.unwritable", subject)}</>;
    case "invalidSignature":
      return <>{t("sede.refusals.invalidSignature", subject)}</>;
    case "confirmationNeeded":
      return <>{t("sede.refusals.confirmationNeeded", subject)}</>;
    case "localBatchSign":
      return <>{t("sede.refusals.localBatchSign", subject)}</>;
    case "siteErrandNotLive":
      return <>{t("sede.refusals.siteErrandNotLive", subject)}</>;
    case "pdfHasUnregisteredSignatures":
      return <>{t("sede.refusals.pdfHasUnregisteredSignatures", subject)}</>;
    case "secretOnTheReaderKeypad":
      return <>{t("sede.refusals.secretOnTheReaderKeypad", subject)}</>;
    case "userCancelled":
      return <>{t("sede.refusals.userCancelled", subject)}</>;
    case "promptFailed":
      return <>{t("sede.refusals.promptFailed", subject)}</>;
    case "unknown":
      return <>{t("sede.refusals.unknown", subject)}</>;
    case "incorrectPin":
      return <>{t("errors.situations.incorrectPin.title")}</>;
    case "pinLocked":
      return <>{t("errors.situations.pinLocked.title")}</>;
    case "tokenAbsent":
      return <>{t("errors.situations.tokenAbsent.title")}</>;
    case "expiredSession":
      return <>{t("errors.situations.expiredSession.title")}</>;
    case "moduleNotFound":
      return <>{t("errors.situations.moduleNotFound.title")}</>;
    case "pkcs12Unreadable":
      return <>{t("errors.situations.pkcs12Unreadable.title")}</>;
    case "keyNotRsa":
      return <>{t("errors.situations.keyNotRsa.title")}</>;
    case "mechanismNotOffered":
      return <>{t("errors.situations.mechanismNotOffered.title")}</>;
    case "notAPdf":
      return <>{t("errors.situations.notAPdf.title")}</>;
    case "documentEncrypted":
      return <>{t("errors.situations.documentEncrypted.title")}</>;
    case "documentCertified":
      return <>{t("errors.situations.documentCertified.title")}</>;
    case "documentUnreadable":
      return <>{t("errors.situations.documentUnreadable.title")}</>;
    case "boxOutOfPage":
      return <>{t("errors.situations.boxOutOfPage.title")}</>;
    case "pageOutOfDocument":
      return <>{t("errors.situations.pageOutOfDocument.title")}</>;
    case "sealMismatch":
      return <>{t("errors.situations.sealMismatch.title")}</>;
    case "bridgeFailed":
      return <>{t("errors.situations.bridgeFailed.title")}</>;
    case "notAFolder":
      return <>{t("errors.situations.notAFolder.title")}</>;
    case "folderUnreadable":
      return <>{t("errors.situations.folderUnreadable.title")}</>;
    case "folderUnwritable":
      return <>{t("errors.situations.folderUnwritable.title")}</>;
    case "noFreeName":
      return <>{t("errors.situations.noFreeName.title")}</>;
  }
}

/** Lo que quien mantiene la sede puede cambiar para que rFirma firme, cuando rFirma se niega. */
function SiteNote({ situation }: { situation: RefusalSituation }) {
  const { t } = useTranslation();

  switch (situation) {
    case "explicitXades":
      return <p className="rf-hint">{t("sede.siteNotes.explicitXades")}</p>;
    case "invoiceMultisignature":
      return <p className="rf-hint">{t("sede.siteNotes.invoiceMultisignature")}</p>;
    case "unsupportedCountersignature":
      return <p className="rf-hint">{t("sede.siteNotes.unsupportedCountersignature")}</p>;
    default:
      return null;
  }
}

/** El título ya dice lo que pasó; nada debajo lo repite. */
function title(outcome: SiteOutcome, t: TFunction): string {
  switch (outcome.kind) {
    case "signed":
      return t("sede.outcome.signedTitle");
    case "batchSigned":
      return t("sede.outcome.batchTitle");
    case "saved":
      return t("sede.outcome.savedTitle");
    case "loaded":
      return t("sede.outcome.loadedTitle");
    case "cancelled":
      return t("sede.outcome.cancelledTitle");
    case "refused":
      return outcome.situation === "portsTaken"
        ? t("sede.outcome.portsTakenTitle")
        : t("sede.outcome.refusedTitle");
    default:
      return t("sede.outcome.refusedTitle");
  }
}
