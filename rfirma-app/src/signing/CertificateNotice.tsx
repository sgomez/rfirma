import { useTranslation } from "react-i18next";
import { AlertIcon } from "../design-system/icons";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { CertificateState } from "./SigningPanel";

/**
 * El aviso de «sin certificados», arriba de la zona que se desliza
 * (docs/design/panel-de-firma.md § Estados). Sin él y sin fallo, no pinta
 * nada: el certificado se elige y se cuenta desde el pie.
 */
export function CertificateNotice({
  state,
  onOpenHelp,
}: {
  state: CertificateState;
  onOpenHelp?: () => void;
}) {
  const { t } = useTranslation();

  if (state.kind === "empty") {
    return (
      <div className="panel__no-certificates">
        <div className="panel__notice-title">
          <AlertIcon />
          <span className="rf-title">{t("panel.certificate.empty.title")}</span>
        </div>
        <p className="rf-prose rf-text-muted">{t("panel.certificate.empty.body")}</p>
      </div>
    );
  }

  if (state.kind === "failed") {
    // El mismo lenguaje que `empty` —título y explicación— con el fallo ya
    // clasificado y su detalle crudo debajo.
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
      </div>
    );
  }

  return null;
}
