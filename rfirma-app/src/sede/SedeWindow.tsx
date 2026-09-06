import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { CloseIcon } from "../design-system/icons";
import { PinDialog } from "../signing/PinDialog";
import type { Errand, SiteErrandPort } from "./errand";
import { SedeConsent } from "./SedeConsent";
import { useWaitingClock } from "./SedeFrame";
import { SedeNoCertificate } from "./SedeNoCertificate";
import { SedeOutcome } from "./SedeOutcome";
import { SedeSigning } from "./SedeSigning";
import { SedeWaiting } from "./SedeWaiting";
import "./SedeWindow.css";

interface SedeWindowProps {
  errands: SiteErrandPort;
}

/**
 * La ventana que abre rFirma cuando una sede electrónica lo invoca por
 * `afirma://` (docs/design/ventana-de-sede.md).
 *
 * **Una ventana con una secuencia, no cinco pantallas**: los cinco momentos
 * comparten las tres regiones fijas —barra de título de 32 px, cuerpo y pie— y
 * lo único que cambia es lo que va dentro. Sin cabecera de aplicación, sin
 * menú, sin bandeja y sin pie de destino: sugerir que hay más dentro invita a
 * buscar cosas que no están.
 *
 * Los tres relojes del trámite viven aquí y no en el backend, porque son
 * conducta de la ventana:
 *
 * - **el retardo de `WAITING_GRACE_MS`** antes de pintar la espera, para
 *   que el camino feliz no dé un fogonazo;
 * - **el umbral de `UNREACHABLE_AFTER_MS`**, el único que hay, para pasar
 *   de «Conectando» a «La petición no ha llegado» — y que **no cierra nada**;
 * - **el cierre a los `OUTCOME_CLOSE_MS`** tras el desenlace (ID-274).
 */
export function SedeWindow({ errands }: SedeWindowProps) {
  const [errand, setErrand] = useState<Errand | null>(null);

  useEffect(() => errands.watch(setErrand), [errands]);

  if (errand === null) return null;
  return <SedeDialog errand={errand} errands={errands} />;
}

/**
 * La ventana ya con trámite. Va aparte para que los relojes de cada momento
 * arranquen al entrar en él y no al montar el árbol: montados en
 * `SedeWindow`, un `useEffect` con `errand` en las dependencias volvería a
 * contar los 15 segundos con cada latido del puerto.
 */
function SedeDialog({ errand, errands }: { errand: Errand; errands: SiteErrandPort }) {
  const { t } = useTranslation();
  const titleId = useId();
  const stage = errand.stage;
  const waiting = useWaitingClock();

  const close = () => void errands.close();
  const cancel = () => void errands.cancel();

  // El retardo tapa la ventana entera, no sólo su cuerpo: el camino feliz abre
  // el canal en ~44 ms, y un marco vacío parpadeando es peor que nada.
  if (stage.kind === "waiting" && waiting === "hidden") return null;

  return (
    /* En el momento del secreto el velo lo pinta `PinDialog`, que trae el suyo:
       dos `.rf-scrim` superpuestos oscurecerían el doble, y la ficha dice que
       ese diálogo no cambia en nada respecto al recorrido local. */
    <div className={`rf-scrim${stage.kind === "secret" ? " sede-window__scrim--clear" : ""}`}>
      <section
        className="sede-window"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        data-stage={stage.kind}
      >
        {/* Un `div` y no un `header`: un punto de referencia «banner» dentro
            de un diálogo sería mentira, y esta ventana **no** tiene cabecera
            de aplicación (ADR-0007). Es una barra de título y nada más. */}
        <div className="rf-row sede-window__bar">
          <span className="rf-body sede-window__brand" id={titleId}>
            {t("app.name")}
          </span>
          <div className="sede-window__spacer" />
          <button
            type="button"
            className="sede-window__close"
            aria-label={t("sede.window.close")}
            /* `close()` sólo cuando la sede **ya tiene su respuesta**, que es
               el desenlace y nada más. En todo lo demás —también en «sin
               certificado utilizable»— irse es abandonar el trámite, y eso es
               `cancel()`: es lo que libera el `idsession`. La salida del pie
               llama al mismo verbo en cada momento; no hay dos puertas que
               hagan cosas distintas. */
            onClick={stage.kind === "outcome" ? close : cancel}
          >
            <CloseIcon size={14} />
          </button>
        </div>

        {stage.kind === "waiting" && (
          <SedeWaiting
            moment={waiting === "unreachable" ? "unreachable" : "connecting"}
            onInstallLocalCa={() => void errands.installLocalCa()}
            onCancel={cancel}
          />
        )}
        {/* **El callejón sin salida es la misma pantalla, sin el reloj**
            (ID-341): cuando el backend ya sabe que no hay canal —ni un puerto
            libre, o la CA local en ninguna parte— esperar treinta segundos a
            un umbral sería enseñar «Conectando» sabiendo que no conecta. La
            reparación no cambia porque tampoco cambia lo que la persona puede
            hacer, y ahí es donde vive la dirección del ajuste de Chrome, que
            se copia y no se pulsa. */}
        {stage.kind === "noChannel" && (
          <SedeWaiting
            moment="unreachable"
            onInstallLocalCa={() => void errands.installLocalCa()}
            onCancel={cancel}
          />
        )}
        {stage.kind === "consent" && (
          <SedeConsent
            origin={errand.origin}
            operation={errand.operation}
            stage={stage}
            onConsent={(certificateId) => void errands.consent(certificateId)}
            onCancel={cancel}
          />
        )}
        {stage.kind === "secret" && (
          <SedeSigning
            origin={errand.origin}
            certificate={stage.certificate}
            phase="signing"
            onCancel={cancel}
          />
        )}
        {stage.kind === "signing" && (
          <SedeSigning
            origin={errand.origin}
            certificate={stage.certificate}
            phase={stage.phase}
            onCancel={cancel}
          />
        )}
        {stage.kind === "outcome" && (
          <SedeOutcome origin={errand.origin} outcome={stage.outcome} onClose={close} />
        )}
        {stage.kind === "noCertificate" && (
          <SedeNoCertificate
            origin={errand.origin}
            reason={stage.reason}
            owned={stage.owned}
            onInstall={() => void errands.installCertificate()}
            onLookAgain={() => void errands.lookAgain()}
            onLeave={cancel}
          />
        )}
      </section>

      {/* El PIN **no tiene pantalla propia** (ID-273): es el mismo diálogo del
          recorrido local, montado encima de la ventana de sede sin una sola
          diferencia. Debajo sigue «Firmando», que es donde el trámite está. */}
      {stage.kind === "secret" && (
        <PinDialog
          certificate={stage.certificate}
          failure={stage.failure}
          onSubmit={(secret) => void errands.submitSecret(secret)}
          onCancel={cancel}
        />
      )}
    </div>
  );
}
