//! 5 · Sin certificado utilizable, porque no hay ninguno o porque la sede los excluyó todos, con sus salidas: instalar otro, volver a buscar o cerrar.

import { useTranslation } from "react-i18next";
import type { NamedFailure } from "../errors/classify";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { NoCertificateReason } from "./errand";
import { SedeBody, useDefaultButton } from "./SedeFrame";

interface SedeNoCertificateProps {
  origin: string | null;
  /** La pide una orden de terminal: nada se dirige a la sede ni queda arreglo que ofrecer. */
  terminal: boolean;
  reason: NoCertificateReason;
  /**
   * Cuántos certificados tiene la persona. Sólo se dice en «excluidos», porque
   * eso es estado de **su** almacén; lo que la sede descartó no se enumera
   * nunca.
   */
  owned: number;
  /** El fallo de instalar, ya clasificado; `null` mientras no se haya intentado. */
  failure: NamedFailure | null;
  onInstall: () => void;
  onLookAgain: () => void;
  /**
   * Salir de aquí. Es `cancel()` del puerto y no `close()`: la sede no ha
   * recibido nada todavía, así que irse deja el `idsession` colgando si no se
   * abandona el trámite (ver `SedeWindow`).
   */
  onLeave: () => void;
}

/**
 * **5 · Sin certificado utilizable.** No es una variante del consentimiento: es
 * otra situación. Allí hay algo que consentir y un certificado que elegir; aquí
 * no hay ni una cosa ni la otra, y el botón principal no puede decir «Firmar».
 *
 * Las dos razones comparten acciones —instalar otro siempre puede arreglarlo,
 * ni siquiera cuando es la sede quien ha excluido los que ya había— y solo
 * cambia el mensaje: **excluidos** cuenta cuántos certificados tiene la
 * persona, porque eso es estado de su almacén, y si el recién instalado
 * tampoco vale ese número sube solo con la pantalla sin cerrarse ni contestar
 * a la sede.
 *
 * `Cerrar` está en el pie **siempre**: es la salida etiquetada, y sin ella
 * quien no quiere instalar nada sólo tiene la cruz de la barra de título.
 * `Volver a buscar` no es del pie sino una **microacción del cuerpo**, que es
 * lo que el criterio de botones de la ficha reserva para `--ghost` en línea.
 */
export function SedeNoCertificate({
  origin,
  terminal,
  reason,
  owned,
  failure,
  onInstall,
  onLookAgain,
  onLeave,
}: SedeNoCertificateProps) {
  const { t } = useTranslation();
  const excluded = reason === "excluded";
  const defaultButton = useDefaultButton();
  const closeOnly = terminal && excluded;

  return (
    <SedeBody
      onEscape={onLeave}
      steadyFooter
      footer={
        <>
          <div className="sede-window__spacer" />
          <button
            ref={closeOnly ? defaultButton : undefined}
            type="button"
            className={closeOnly ? "rf-btn rf-btn--primary" : "rf-btn rf-btn--ghost"}
            onClick={onLeave}
          >
            {t("actions.close")}
          </button>
          {!closeOnly && (
            <button
              ref={defaultButton}
              type="button"
              className="rf-btn rf-btn--primary"
              onClick={onInstall}
            >
              {t("sede.noCertificate.install")}
            </button>
          )}
        </>
      }
    >
      <div className="rf-stack sede-no-certificate">
        <p className="rf-title sede-no-certificate__title">
          {excluded
            ? terminal
              ? t("sede.noCertificate.terminalExcludedTitle", { count: owned })
              : t("sede.noCertificate.excludedTitle", {
                  count: owned,
                  origin: origin ?? t("sede.origin.unknown"),
                })
            : t("sede.noCertificate.noneTitle")}
        </p>
        {!terminal && (
          <p className="rf-prose rf-text-muted">
            {excluded
              ? t("sede.noCertificate.excludedBody")
              : origin === null
                ? t("sede.noCertificate.noneBodyUnknownOrigin")
                : t("sede.noCertificate.noneBody", { origin })}
          </p>
        )}
        {!terminal && !excluded && <p className="rf-hint">{t("sede.noCertificate.noneHint")}</p>}
        {failure !== null && (
          <ErrorNotice situation={failure.situation} technicalDetail={failure.detail} />
        )}
        {/* La microacción va aquí, pegada a lo que arregla, y no en el pie:
            se pulsa cuando se acaba de instalar uno con la ventana abierta. */}
        {!closeOnly && (
          <div className="rf-row sede-no-certificate__look-again">
            <button type="button" className="rf-btn rf-btn--ghost" onClick={onLookAgain}>
              {t("sede.noCertificate.lookAgain")}
            </button>
          </div>
        )}
      </div>
    </SedeBody>
  );
}
