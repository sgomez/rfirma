import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { CopyIcon } from "../design-system/icons";
import { OUTCOME_CLOSE_MS, type RefusalSituation, type SiteOutcome } from "./errand";
import { SedeBody, useOutcomeClock } from "./SedeFrame";

interface SedeOutcomeProps {
  origin: string | null;
  outcome: SiteOutcome;
  onClose: () => void;
}

/**
 * **4 · Desenlace.** Tres desenlaces, y en los tres **la sede ya ha recibido su
 * respuesta**: los dos canales van desacompasados a propósito (#316).
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
export function SedeOutcome({ origin, outcome, onClose }: SedeOutcomeProps) {
  const { t } = useTranslation();
  useOutcomeClock(onClose);

  return (
    <SedeBody
      steadyFooter
      footer={
        <>
          <p className="rf-hint sede-outcome__auto-close">
            {t("sede.outcome.autoClose", { seconds: OUTCOME_CLOSE_MS / 1000 })}
          </p>
          <div className="sede-window__spacer" />
          <button type="button" className="rf-btn rf-btn--primary" onClick={onClose}>
            {t("actions.close")}
          </button>
        </>
      }
    >
      <div className="rf-stack sede-outcome">
        <p className="rf-title sede-outcome__title">{title(outcome, t)}</p>
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
        {outcome.kind === "refused" && (
          <>
            <p className="rf-prose">
              <RefusalSentence situation={outcome.situation} origin={origin} />
            </p>
            <p className="rf-hint">{t("sede.outcome.refusedNote")}</p>
            <div className="rf-row rf-gap-xs sede-outcome__detail">
              <span className="rf-label">{t("sede.outcome.detail")}</span>
              <code className="rf-body sede-outcome__detail-text">{outcome.detail}</code>
              <button
                type="button"
                className="rf-btn rf-btn--ghost"
                onClick={() => void navigator.clipboard.writeText(outcome.detail)}
              >
                <CopyIcon size={14} />
                {t("actions.copy")}
              </button>
            </div>
          </>
        )}
      </div>
    </SedeBody>
  );
}

/**
 * La incompatibilidad, enunciada nombrando el origen y **sin acusar a nadie**:
 * se dice el hecho y quien lee saca la conclusión.
 *
 * Cada clave se escribe **entera**, sin plantilla: una clave
 * ensamblada con plantilla no la ve ni `extract --ci` ni `status --unused`, y
 * las seis saldrían como claves sin uso (`src/AGENTS.md`).
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
    case "errandInFlight":
      return <>{t("sede.refusals.errandInFlight", subject)}</>;
    default:
      // Una situación nueva en el catálogo cae aquí hasta que se le escriba su
      // rama: `unknown` dice lo que pasa sin fingir que se sabe cuál era.
      return <>{t("sede.refusals.unknown", subject)}</>;
  }
}

/** El título ya dice lo que pasó; nada debajo lo repite. */
function title(outcome: SiteOutcome, t: TFunction): string {
  switch (outcome.kind) {
    case "signed":
      return t("sede.outcome.signedTitle");
    case "cancelled":
      return t("sede.outcome.cancelledTitle");
    default:
      return t("sede.outcome.refusedTitle");
  }
}
