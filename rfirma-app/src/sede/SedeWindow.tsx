import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Errand, SiteErrandPort } from "./errand";
import { SedeConfirm } from "./SedeConfirm";
import { SedeConsent } from "./SedeConsent";
import { SedeNoCertificate } from "./SedeNoCertificate";
import { SedeOutcome } from "./SedeOutcome";
import { SedeSigning } from "./SedeSigning";
import { SedeTransfer } from "./SedeTransfer";
import { SedeWaiting } from "./SedeWaiting";
import "./SedeWindow.css";

interface SedeWindowProps {
  errands: SiteErrandPort;
}

/**
 * La ventana que abre rFirma cuando una sede electrónica lo invoca por
 * `afirma://` (docs/design/ventana-de-sede.md).
 *
 * **Una ventana con una secuencia, no una pantalla por momento**: todos los
 * momentos comparten las dos regiones fijas —cuerpo y pie— y lo único que cambia es lo
 * que va dentro. Sin cabecera de aplicación, sin menú, sin bandeja y sin pie de
 * destino: sugerir que hay más dentro invita a buscar cosas que no están.
 *
 * **La barra de título es la del sistema operativo**, no una pintada aquí: una
 * de mentira no la mueve el gestor de ventanas, así que la ventana no se podía
 * ni arrastrar. Con ella vienen gratis el título, la cruz, el menú del gestor y
 * el arrastre; y cerrar por la cruz llega igual al backend, que ya trata
 * `CloseRequested` sobre esta ventana como abandonar el trámite (ID-340).
 *
 * El desenlace se cierra solo a los `OUTCOME_CLOSE_MS` (ID-274). El momento
 * de «no ha llegado» lo decide el backend con su reloj de respaldo y lo publica
 * como un momento más.
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
  const stage = errand.stage;

  const close = () => void errands.close();
  const cancel = () => void errands.cancel();

  return (
    <div className="rf-scrim">
      <section
        className="sede-window"
        role="dialog"
        aria-modal="true"
        aria-label={t("app.name")}
        data-stage={stage.kind}
      >
        {stage.kind === "waiting" && (
          <SedeWaiting
            moment="connecting"
            onInstallLocalCa={() => void errands.installLocalCa()}
            onCancel={cancel}
          />
        )}
        {(stage.kind === "unreachable" || stage.kind === "noChannel") && (
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
        {stage.kind === "confirming" && (
          <SedeConfirm
            messageCode={stage.messageCode}
            onConfirm={() => errands.confirmSignatures()}
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
        {/* Guardar y cargar **no preguntan en esta ventana**: la orden abre el
            diálogo del portal en cuanto el momento llega, y aquí sólo se
            nombra el fichero (ADR-0011). */}
        {(stage.kind === "saving" || stage.kind === "loading") && <SedeTransfer transfer={stage} />}
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
    </div>
  );
}
