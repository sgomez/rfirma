//! 4 · El desenlace con el esqueleto del consentimiento compacto: el nombre de la sede, el icono junto al título, y en el rechazo su frase y el detalle copiable.

import type { TFunction } from "i18next";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Admonition } from "../design-system/Admonition";
import { Button } from "../design-system/Button";
import { Icon } from "../design-system/icons";
import { Row } from "../design-system/Row";
import { useDefaultButton } from "../design-system/useDefaultButton";
import { errorText } from "../errors/errorMessage";
import {
  isSedeRefusal,
  OUTCOME_CLOSE_MS,
  REFUSAL_ACTION_OF,
  type RefusalSituation,
  type SiteOutcome,
} from "./errand";
import { SedeBody, useOutcomeClock } from "./SedeFrame";
import { SedeOrigin } from "./SedeOrigin";
import "./SedeWindow.css";
import "./SedeEnding.css";

interface SedeEndingProps {
  origin: string | null;
  outcome: SiteOutcome;
  onClose: () => void;
  onOpenHelp?: () => void;
  onReport?: () => void;
}

/** El desenlace: quién lo pidió arriba, lo que pasó en medio y «Cerrar» con su cuenta atrás donde estaba «Firmar». */
export function SedeEnding({ origin, outcome, onClose, onOpenHelp, onReport }: SedeEndingProps) {
  const { t } = useTranslation();
  const carriesHelp = outcome.kind === "refused" && outcome.situation === "unknown";
  const asksToAct =
    outcome.kind === "refused" &&
    isSedeRefusal(outcome.situation) &&
    REFUSAL_ACTION_OF[outcome.situation] === "closeOther";
  const staysOpen =
    carriesHelp || asksToAct || (outcome.kind === "refused" && onReport !== undefined);
  useOutcomeClock(onClose, !staysOpen);
  const seconds = useClosingSeconds(!staysOpen);
  const closeButton = useDefaultButton();

  return (
    <SedeBody
      flush
      steadyFooter
      primary={closeButton}
      onEscape={onClose}
      footer={
        <>
          {outcome.kind === "refused" && onReport !== undefined && (
            <Button variant="ghost" onClick={onReport}>
              Informar del problema
            </Button>
          )}
          <Button
            ref={closeButton}
            variant="primary"
            className="sede-ending__close"
            onClick={onClose}
          >
            {!staysOpen && seconds > 0
              ? t("sede.outcome.closeIn", { seconds })
              : t("actions.close")}
          </Button>
        </>
      }
    >
      <div className="sede-ending">
        <SedeOrigin origin={origin} asks={null} />
        <div className="sede-ending__body">
          <Row gap="xs" className="sede-outcome__head">
            <span className="sede-outcome__icon">
              <OutcomeIcon kind={outcome.kind} />
            </span>
            <p className="rf-title sede-ending__title">{outcomeTitle(outcome, t)}</p>
          </Row>
          {outcome.kind === "refused" && (
            <p className="rf-prose sede-ending__statement">
              <RefusalStatement situation={outcome.situation} />
            </p>
          )}
          {outcome.kind === "refused" && (
            <EndingDetail situation={outcome.situation} detail={outcome.detail} />
          )}
          {carriesHelp && (
            <Row gap="xs" className="sede-outcome__report">
              <p className="rf-hint">{t("sede.outcome.reportHint")}</p>
              <Button variant="ghost" className="sede-outcome__help" onClick={() => onOpenHelp?.()}>
                <Icon name="externalLink" size={14} />
                {t("header.help")}
              </Button>
            </Row>
          )}
        </div>
      </div>
    </SedeBody>
  );
}

function EndingDetail({ situation, detail }: { situation: RefusalSituation; detail: string }) {
  const { t } = useTranslation();
  const note = siteNote(situation, t);
  const copiedDetail = note === null ? detail : `${note}\n\n${detail}`;

  return (
    <Admonition
      tone="neutral"
      className="sede-ending__detail"
      title={t("sede.outcome.technicalDetail")}
      action={
        <Button variant="ghost" onClick={() => void navigator.clipboard.writeText(copiedDetail)}>
          <Icon name="copy" size={14} />
          {t("actions.copy")}
        </Button>
      }
    >
      {note !== null && <p className="rf-body">{note}</p>}
      {/* biome-ignore lint/a11y/noNoninteractiveTabindex: la región con desplazamiento tiene que poder enfocarse para leerla con el teclado. */}
      <code className="rf-body sede-ending__detail-text" tabIndex={0}>
        {detail}
      </code>
    </Admonition>
  );
}

function useClosingSeconds(enabled: boolean): number {
  const [remaining, setRemaining] = useState(OUTCOME_CLOSE_MS / 1000);

  useEffect(() => {
    if (!enabled || remaining === 0) return;
    const timer = setTimeout(() => setRemaining((seconds) => seconds - 1), 1000);
    return () => clearTimeout(timer);
  }, [enabled, remaining]);

  return remaining;
}

/** Los tres del artboard: visto, aspa y triángulo, uno por desenlace. */
function OutcomeIcon({ kind }: { kind: SiteOutcome["kind"] }) {
  switch (kind) {
    case "signed":
    case "batchSigned":
    case "saved":
    case "loaded":
      return <Icon name="success" size={24} />;
    case "cancelled":
      return <Icon name="failure" size={24} />;
    default:
      return <Icon name="warning" size={24} />;
  }
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

/** Por qué rFirma no hace una firma que la sede pide de forma insegura o imposible; en otro rechazo, qué hacer. */
function RefusalStatement({ situation }: { situation: RefusalSituation }) {
  const { t } = useTranslation();

  switch (situation) {
    case "sha1":
      return <>{t("sede.refusalCauses.sha1")}</>;
    case "invoiceMultisignature":
      return <>{t("sede.refusalCauses.invoiceMultisignature")}</>;
    case "unsupportedCountersignature":
      return <>{t("sede.refusalCauses.unsupportedCountersignature")}</>;
    default:
      return <RefusalSentence situation={situation} />;
  }
}

/** Lo que quien mantiene la sede puede cambiar para que rFirma firme, cuando rFirma se niega. */
function siteNote(situation: RefusalSituation, t: TFunction): string | null {
  switch (situation) {
    case "sha1":
      return t("sede.siteNotes.sha1");
    case "invoiceMultisignature":
      return t("sede.siteNotes.invoiceMultisignature");
    case "unsupportedCountersignature":
      return t("sede.siteNotes.unsupportedCountersignature");
    default:
      return null;
  }
}

/** El título ya dice lo que pasó; nada debajo lo repite. */
function outcomeTitle(outcome: SiteOutcome, t: TFunction): string {
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
