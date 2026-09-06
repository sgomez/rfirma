import { useTranslation } from "react-i18next";
import type { Certificate } from "../signing/certificate";
import type { SigningPhase } from "./errand";
import { SedeBody } from "./SedeFrame";

interface SedeSigningProps {
  origin: string | null;
  /** Con quién se está firmando: lo único que la persona acaba de elegir. */
  certificate: Certificate;
  phase: SigningPhase;
  onCancel: () => void;
}

/**
 * Lo lejos que está cada tramo. **No son porcentajes de nada**: si los dos
 * marcaran lo mismo, «firmando» y «devolviendo» se verían igual de lejos, que
 * es justo lo que la barra viene a distinguir.
 */
const PROGRESS: Record<SigningPhase, number> = { signing: 45, returning: 88 };

/**
 * **3 · Firmando.** Lo que la ventana enseña entre que la persona acepta y que
 * la firma vuelve a la sede. Hoy AutoFirma no enseña nada, y ése es el fallo.
 *
 * **No es el diálogo de progreso de la ventana principal**: allí se listan las
 * tres fases de la trifásica porque hay un fichero pedido y el reparto explica
 * por qué tarda. Aquí no hay destino que enseñar, y contar «prefirma» sería
 * estado interno del motor.
 *
 * **Cero acciones principales** en toda la pantalla. Mientras rFirma firma,
 * `Cancelar` es limpio —la sede no ha recibido nada—; cuando la respuesta ya va
 * de camino no hay nada que cancelar, y **el pie se queda vacío** en vez de
 * ofrecer un botón que mentiría.
 */
export function SedeSigning({ origin, certificate, phase, onCancel }: SedeSigningProps) {
  const { t } = useTranslation();
  const returning = phase === "returning";

  return (
    <SedeBody
      steadyFooter
      footer={
        returning ? null : (
          <>
            <div className="sede-window__spacer" />
            <button type="button" className="rf-btn rf-btn--ghost" onClick={onCancel}>
              {t("actions.cancel")}
            </button>
          </>
        )
      }
    >
      <div className="rf-stack sede-signing">
        <p className="rf-title sede-signing__title">
          {returning
            ? origin === null
              ? t("sede.returning.titleUnknownOrigin")
              : t("sede.returning.title", { origin })
            : t("sede.signing.title")}
        </p>
        <p className="rf-prose rf-text-muted">
          {returning
            ? t("sede.returning.body")
            : t("sede.signing.with", {
                holder: certificate.holderName,
                idNumber: certificate.idNumber,
              })}
        </p>
        <div
          className="sede-signing__track"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={PROGRESS[phase]}
          aria-label={t("sede.signing.title")}
        >
          <div className="sede-signing__bar" style={{ width: `${PROGRESS[phase]}%` }} />
        </div>
      </div>
    </SedeBody>
  );
}
