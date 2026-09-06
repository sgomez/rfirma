import { useTranslation } from "react-i18next";
import type { NoCertificateReason } from "./errand";
import { SedeBody } from "./SedeFrame";

interface SedeNoCertificateProps {
  origin: string | null;
  reason: NoCertificateReason;
  /**
   * Cuántos certificados tiene la persona. Sólo se dice en «excluidos», porque
   * eso es estado de **su** almacén; lo que la sede descartó no se enumera
   * nunca (ID-277).
   */
  owned: number;
  onInstall: () => void;
  onLookAgain: () => void;
  onClose: () => void;
}

/**
 * **5 · Sin certificado utilizable.** No es una variante del consentimiento: es
 * otra situación. Allí hay algo que consentir y un certificado que elegir; aquí
 * no hay ni una cosa ni la otra, y el botón principal no puede decir «Firmar».
 *
 * Las dos opciones **se tienen que sentir distintas porque la salida es
 * distinta** (ID-278):
 *
 * - **no tienes ninguno** tiene arreglo, y el arreglo no depende de la sede: hay
 *   acción principal —`Instalar un certificado…`— y la microacción `Volver a
 *   buscar`, por si se instaló con la ventana ya abierta;
 * - **la sede los ha excluido todos** no lo tiene, porque quien decide es la
 *   sede: la pantalla se queda **sin acción principal** y la única salida es
 *   `Cerrar`.
 */
export function SedeNoCertificate({
  origin,
  reason,
  owned,
  onInstall,
  onLookAgain,
  onClose,
}: SedeNoCertificateProps) {
  const { t } = useTranslation();
  const excluded = reason === "excluded";

  return (
    <SedeBody
      steadyFooter
      footer={
        <>
          <div className="sede-window__spacer" />
          {!excluded && (
            <button type="button" className="rf-btn rf-btn--ghost" onClick={onLookAgain}>
              {t("sede.noCertificate.lookAgain")}
            </button>
          )}
          <button
            type="button"
            className={`rf-btn rf-btn--${excluded ? "ghost" : "primary"}`}
            onClick={excluded ? onClose : onInstall}
          >
            {excluded ? t("actions.close") : t("sede.noCertificate.install")}
          </button>
        </>
      }
    >
      <div className="rf-stack sede-no-certificate">
        <p className="rf-title sede-no-certificate__title">
          {excluded
            ? t("sede.noCertificate.excludedTitle", {
                count: owned,
                origin: origin ?? t("sede.origin.unknown"),
              })
            : t("sede.noCertificate.noneTitle")}
        </p>
        <p className="rf-prose rf-text-muted">
          {excluded
            ? t("sede.noCertificate.excludedBody")
            : origin === null
              ? t("sede.noCertificate.noneBodyUnknownOrigin")
              : t("sede.noCertificate.noneBody", { origin })}
        </p>
        {!excluded && <p className="rf-hint">{t("sede.noCertificate.noneHint")}</p>}
      </div>
    </SedeBody>
  );
}
