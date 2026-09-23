import { useTranslation } from "react-i18next";
import { AlertIcon } from "../design-system/icons";
import { ErrorNotice } from "../errors/ErrorNotice";
import { CertificateSelect, statusWarning } from "./CertificateSelect";
import type { Certificate } from "./certificate";
import { isUsable } from "./certificate";
import type { CertificateState } from "./SigningPanel";

/** El certificado, en sus cinco estados. */
export function CertificateBlock({
  state,
  onChoose,
  onRetry,
  onChooseModule,
  onOpenHelp,
}: {
  state: CertificateState;
  onChoose: (certificate: Certificate) => void;
  onRetry: () => void;
  onChooseModule: () => void;
  onOpenHelp?: () => void;
}) {
  const { t, i18n } = useTranslation();

  if (state.kind === "loading") {
    return (
      <div className="panel__skeletons">
        <p className="rf-prose rf-text-muted">{t("panel.certificate.loading")}</p>
        <span className="panel__skeleton" />
        <span className="panel__skeleton" />
      </div>
    );
  }

  if (state.kind === "empty") {
    return (
      <div className="panel__no-certificates">
        <div className="panel__notice-title">
          <AlertIcon />
          <span className="rf-title">{t("panel.certificate.empty.title")}</span>
        </div>
        <p className="rf-prose rf-text-muted">{t("panel.certificate.empty.body")}</p>
        <div className="rf-row rf-gap-xs panel__no-certificates-actions">
          <button type="button" className="rf-btn rf-btn--secondary panel__retry" onClick={onRetry}>
            {t("panel.certificate.retry")}
          </button>
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onChooseModule}>
            {t("panel.certificate.otherModule")}
          </button>
        </div>
      </div>
    );
  }

  if (state.kind === "failed") {
    // El mismo lenguaje que `empty` —título, explicación, botón de volver a
    // buscar— con texto propio, más el fallo ya clasificado con su detalle
    // crudo debajo: quien firma tiene que poder distinguir «mete la tarjeta»
    // de «algo va mal» (ID-10).
    return (
      <div className="panel__no-certificates">
        <div className="panel__notice-title">
          <AlertIcon />
          <span className="rf-title">{t("panel.certificate.failed.title")}</span>
        </div>
        <p className="rf-prose rf-text-muted">{t("panel.certificate.failed.body")}</p>
        <ErrorNotice
          situation={state.failure.situation}
          technicalDetail={state.failure.detail}
          onOpenHelp={onOpenHelp}
        />
        <div className="rf-row rf-gap-xs panel__no-certificates-actions">
          <button type="button" className="rf-btn rf-btn--secondary panel__retry" onClick={onRetry}>
            {t("panel.certificate.retry")}
          </button>
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onChooseModule}>
            {t("panel.certificate.otherModule")}
          </button>
        </div>
      </div>
    );
  }

  // Elegido o no, el hueco lo ocupa el mismo desplegable: cambiar de
  // certificado es volver a abrirlo, y por eso el botón `Cambiar` de la tarjeta
  // ya no existe.
  const chosen = state.kind === "chosen" ? state.certificate : null;
  return (
    <>
      <CertificateSelect certificates={state.certificates} chosen={chosen} onChoose={onChoose} />
      {/* Con uno solo se elige solo, así que puede quedar puesto uno que no
          sirve: el aviso se queda debajo del disparador, donde estaba en la
          tarjeta. Elegido de la lista esto no se ve nunca, porque las filas
          inutilizables no se dejan elegir. */}
      {chosen !== null && !isUsable(chosen.status) && (
        <p className="rf-prose panel__certificate-warning" role="alert">
          {statusWarning(chosen.status, i18n.language, t)}
        </p>
      )}
    </>
  );
}
