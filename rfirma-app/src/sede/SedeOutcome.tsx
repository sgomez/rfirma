//! 4 · El desenlace: firmado, lote entregado, cancelado, guardado, cargado o rechazado, con el documento recién firmado y el detalle copiable del rechazo.

import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import {
  AlertIcon,
  CheckCircleIcon,
  CopyIcon,
  CrossCircleIcon,
  ExternalLinkIcon,
  FileIcon,
} from "../design-system/icons";
import { Row } from "../design-system/Row";
import { Stack } from "../design-system/Stack";
import { errorText } from "../errors/errorMessage";
import { formatSize } from "../signing/SigningPanel";
import {
  isSedeRefusal,
  OUTCOME_CLOSE_MS,
  REFUSAL_ACTION_OF,
  type RefusalSituation,
  type SiteDocument,
  type SiteOutcome,
} from "./errand";
import { SedeBody, useDefaultButton, useOutcomeClock } from "./SedeFrame";

interface SedeOutcomeProps {
  origin: string | null;
  outcome: SiteOutcome;
  onClose: () => void;
  onOpenHelp?: () => void;
}

/**
 * **4 · Desenlace.** En todos ellos **la sede ya ha recibido su respuesta**: los
 * dos canales van desacompasados a propósito. Salvo el rechazo de la
 * petición misma, que sale al cerrar, como el diálogo de error del original.
 *
 * El **rechazo** cubre los del transporte, que ocurren antes de que haya nada
 * que consentir. El argumento para enseñarlo no es que la persona pueda
 * arreglarlo —casi nunca puede—: es que acaba de arrancarse un programa en su
 * equipo a petición de una web, y un rFirma que aparece y desaparece en
 * silencio es indistinguible de uno roto. Se le dice qué puede hacer, y el
 * detalle copiable nombra la causa.
 *
 * **Se cierra sola a los quince segundos**, no a los cinco: con cinco no daba
 * tiempo a leer, y el caso que lo decide es el rechazo, donde irse sola
 * reproduciría el síntoma que el aviso venía a evitar.
 */
export function SedeOutcome({ origin, outcome, onClose, onOpenHelp }: SedeOutcomeProps) {
  const { t } = useTranslation();
  const carriesHelp = outcome.kind === "refused" && outcome.situation === "unknown";
  const asksToAct =
    outcome.kind === "refused" &&
    isSedeRefusal(outcome.situation) &&
    REFUSAL_ACTION_OF[outcome.situation] === "closeOther";
  const staysOpen = carriesHelp || asksToAct;
  useOutcomeClock(onClose, !staysOpen);
  const closeButton = useDefaultButton();

  const openHelp = () => {
    onOpenHelp?.();
  };

  return (
    <SedeBody
      onEscape={onClose}
      steadyFooter
      footer={
        <>
          {!staysOpen && (
            <p className="rf-hint sede-outcome__auto-close">
              {t("sede.outcome.autoClose", { seconds: OUTCOME_CLOSE_MS / 1000 })}
            </p>
          )}
          <div className="sede-window__spacer" />
          <Button ref={closeButton} variant="primary" onClick={onClose}>
            {t("actions.close")}
          </Button>
        </>
      }
    >
      <Stack className="sede-outcome">
        <Row gap="xs" className="sede-outcome__head">
          <span className="sede-outcome__icon">
            <OutcomeIcon kind={outcome.kind} />
          </span>
          <p className="rf-title sede-outcome__title">{title(outcome, t)}</p>
        </Row>
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
            <RefusalCause situation={outcome.situation} />
            <p className="rf-prose">
              <RefusalSentence situation={outcome.situation} />
            </p>
            <p className="rf-hint">{t("sede.outcome.refusedNote")}</p>
            {outcome.situation === "sha1" && (
              <p className="rf-prose">{t("sede.refusalHints.sha1")}</p>
            )}
            <SiteNote situation={outcome.situation} />
            <Stack gap="xs" className="sede-outcome__detail">
              <Row gap="xs" className="sede-outcome__detail-head">
                <span className="rf-label">{t("errors.technicalDetail")}</span>
                <Button
                  variant="ghost"
                  className="sede-outcome__copy"
                  onClick={() => void navigator.clipboard.writeText(outcome.detail)}
                >
                  <CopyIcon size={14} />
                  {t("actions.copy")}
                </Button>
              </Row>
              {/* biome-ignore lint/a11y/noNoninteractiveTabindex: la región con desplazamiento tiene que poder enfocarse para leerla con el teclado. */}
              <code className="rf-body sede-outcome__detail-text" tabIndex={0}>
                {outcome.detail}
              </code>
            </Stack>
            {outcome.situation === "unknown" && (
              <Row gap="xs" className="sede-outcome__report">
                <p className="rf-hint">{t("sede.outcome.reportHint")}</p>
                <Button variant="ghost" className="sede-outcome__help" onClick={openHelp}>
                  <ExternalLinkIcon size={14} />
                  {t("header.help")}
                </Button>
              </Row>
            )}
          </>
        )}
      </Stack>
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
    <Row gap="xs" className="sede-outcome__document">
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
    </Row>
  );
}

/** Lo que puede hacer la persona; cada clave va entera para que la vea `extract`. */
function RefusalSentence({ situation }: { situation: RefusalSituation }) {
  const { t } = useTranslation();

  if (!isSedeRefusal(situation)) return <>{errorText(situation, t).title}</>;
  switch (REFUSAL_ACTION_OF[situation]) {
    case "retry":
      return <>{t("sede.refusals.retry")}</>;
    case "contactSite":
      return <>{t("sede.refusals.contactSite")}</>;
    case "closeOther":
      return <>{t("sede.refusals.closeOther")}</>;
    case "otherCertificate":
      return <>{t("sede.refusals.otherCertificate")}</>;
  }
}

/** Por qué rFirma no hace una firma que la sede pide de forma insegura o imposible. */
function RefusalCause({ situation }: { situation: RefusalSituation }) {
  const { t } = useTranslation();

  switch (situation) {
    case "sha1":
    case "sha1InXml":
      return <p className="rf-prose">{t("sede.refusalCauses.sha1")}</p>;
    case "explicitXades":
      return <p className="rf-prose">{t("sede.refusalCauses.explicitXades")}</p>;
    case "invoiceMultisignature":
      return <p className="rf-prose">{t("sede.refusalCauses.invoiceMultisignature")}</p>;
    case "unsupportedCountersignature":
      return <p className="rf-prose">{t("sede.refusalCauses.unsupportedCountersignature")}</p>;
    default:
      return null;
  }
}

/** Lo que quien mantiene la sede puede cambiar para que rFirma firme, cuando rFirma se niega. */
function SiteNote({ situation }: { situation: RefusalSituation }) {
  const { t } = useTranslation();

  switch (situation) {
    case "sha1":
    case "sha1InXml":
      return <p className="rf-hint">{t("sede.siteNotes.sha1")}</p>;
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
    case "batchSigned":
      return t("sede.outcome.signedTitle");
    case "saved":
      return t("sede.outcome.savedTitle");
    case "loaded":
      return t("sede.outcome.loadedTitle");
    case "cancelled":
      return t("sede.outcome.cancelledTitle");
    default:
      return t("sede.outcome.refusedTitle");
  }
}
