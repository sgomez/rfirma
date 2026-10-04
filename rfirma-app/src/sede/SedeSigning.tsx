//! 3 · Los dos tramos de la firma, firmar y devolver a la sede, sin nombrar ninguna fase del motor.

import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { ProgressBar } from "../design-system/ProgressBar";
import { Stack } from "../design-system/Stack";
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
      onEscape={returning ? undefined : onCancel}
      steadyFooter
      footer={
        returning ? null : (
          <>
            <div className="sede-window__spacer" />
            <Button variant="ghost" onClick={onCancel}>
              {t("actions.cancel")}
            </Button>
          </>
        )
      }
    >
      <Stack className="sede-signing">
        <p className="rf-title sede-signing__title">
          {returning
            ? origin === null
              ? t("sede.returning.titleUnknownOrigin")
              : t("sede.returning.title", { origin })
            : t("panel.footer.signing")}
        </p>
        <p className="rf-prose rf-text-muted">
          {returning
            ? t("sede.returning.body")
            : t("sede.signing.with", {
                holder: certificate.holderName,
                idNumber: certificate.idNumber,
              })}
        </p>
        <ProgressBar
          variant="framed"
          value={PROGRESS[phase]}
          aria-label={t("panel.footer.signing")}
        />
      </Stack>
    </SedeBody>
  );
}
