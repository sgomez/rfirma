//! La pieza de dominio que pinta un certificado: titular o representado, línea con el NIF, almacenes, caducidad y motivo si no sirve para firmar.

import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { Badge } from "../design-system/Badge";
import { ClockIcon, RevokedIcon } from "../design-system/icons";
import type { Certificate } from "./certificate";
import { certificateHeadline, certificateSubtitle, expiryMonthYear, isUsable } from "./certificate";
import "./CertificateCard.css";

type Store = Certificate["stores"][number];

interface CertificateCardProps {
  certificate: Certificate;
}

/** Cómo se ve un certificado, igual en el selector y en la lista de Preferencias. */
export function CertificateCard({ certificate }: CertificateCardProps) {
  const { t, i18n } = useTranslation();
  const usable = isUsable(certificate.status);
  return (
    <span className={`certificate-card${usable ? "" : " certificate-card--unusable"}`}>
      <span className="certificate-card__headline">{certificateHeadline(certificate)}</span>
      <span className="rf-body certificate-card__line">{certificateSubtitle(certificate, t)}</span>
      <span className="certificate-card__meta">
        {certificate.stores.map((store) => (
          <Badge key={store} className="certificate-card__store">
            {storeLabel(store, t)}
          </Badge>
        ))}
        {certificate.status.kind === "valid" && (
          <span className="rf-body rf-text-muted certificate-card__expiry">
            {t("panel.certificate.expiresIn", {
              date: expiryMonthYear(certificate.status.notAfter),
            })}
          </span>
        )}
      </span>
      {!usable && (
        <span className="certificate-card__reason">
          <StatusIcon status={certificate.status} />
          <span className="rf-body">
            {shortStatusWarning(certificate.status, i18n.language, t)}
          </span>
        </span>
      )}
    </span>
  );
}

/** El rótulo de un almacén, del catálogo. */
export function storeLabel(store: Store, t: TFunction): string {
  switch (store) {
    case "dnie":
      return t("panel.certificate.stores.dnie");
    case "card":
      return t("status.storeBrands.card");
    case "firefox":
      return t("status.storeBrands.firefox");
    case "chrome":
      return t("panel.certificate.stores.chrome");
    case "nssdb":
      return t("panel.certificate.stores.nssdb");
    case "installed":
      return t("panel.certificate.stores.installed");
    case "windows":
      return t("status.storeBrands.windows");
  }
}

function StatusIcon({ status }: { status: Certificate["status"] }) {
  switch (status.kind) {
    case "expired":
    case "notYetValid":
      return <ClockIcon />;
    case "revoked":
      return <RevokedIcon />;
    default:
      return null;
  }
}

/** Por qué no se puede firmar con este certificado, en la frase corta de su fila. */
export function shortStatusWarning(
  status: Certificate["status"],
  locale: string,
  t: TFunction,
): string {
  switch (status.kind) {
    case "expired":
      return t("panel.certificate.expiredShort", {
        date: new Intl.DateTimeFormat(locale, { dateStyle: "long" }).format(status.notAfter * 1000),
      });
    case "notYetValid":
      return t("panel.certificate.notYetValidShort");
    case "revoked":
      return t("panel.certificate.revokedShort", { reason: status.reason });
    default:
      return t("panel.certificate.unreadableShort");
  }
}
