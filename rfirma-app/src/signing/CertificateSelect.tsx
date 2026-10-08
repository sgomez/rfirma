//! El selector de certificado: el `Combobox` con la tarjeta de certificado por opción, primer bloque del panel de firma y el mismo en la sede.

import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { Combobox, type ComboboxOption } from "../design-system/Combobox";
import { SpinnerIcon } from "../design-system/icons";
import { CertificateCard, shortStatusWarning, storeLabel } from "./CertificateCard";
import type { Certificate } from "./certificate";
import {
  certificateCompactSubtitle,
  certificateHeadline,
  groupCertificates,
  isUsable,
} from "./certificate";
import "./CertificateSelect.css";

interface CertificateSelectProps {
  certificates: readonly Certificate[];
  /** El elegido, o `null` mientras no hay ninguno. */
  chosen: Certificate | null;
  onChoose: (certificate: Certificate) => void;
  /** El alto máximo de la lista abierta, en px; la ventana lo recorta si no cabe. */
  listMaxHeight?: number;
  /** Mientras se listan los certificados: la caja lo dice y no se abre. */
  searching?: boolean;
  /** Sin ninguno que elegir: la caja se queda en su sitio, desactivada, y lo dice. */
  absent?: "empty" | "failed";
  disabled?: boolean;
  defaultOpen?: boolean;
}

/** Con qué certificado se firma: la caja de dos líneas que al abrirse es un buscador (docs/design/panel-de-firma.md). */
export function CertificateSelect({
  certificates,
  chosen,
  onChoose,
  listMaxHeight = 480,
  searching = false,
  absent,
  disabled = false,
  defaultOpen = false,
}: CertificateSelectProps) {
  const { t, i18n } = useTranslation();
  const grouped = groupCertificates(certificates);
  const availableLabel = t("panel.certificate.groups.available");
  const unusableLabel = t("panel.certificate.groups.cannotUse");

  const optionOf = (certificate: Certificate, group: string): ComboboxOption<Certificate> => {
    const usable = isUsable(certificate.status);
    return {
      id: certificate.id,
      item: certificate,
      keywords: keywordsOf(certificate, t),
      group,
      disabled: !usable,
      title: usable
        ? rowTooltip(certificate, i18n.language, t)
        : shortStatusWarning(certificate.status, i18n.language, t),
    };
  };
  const options = [
    ...grouped.available.map((certificate) => optionOf(certificate, availableLabel)),
    ...grouped.unusable.map((certificate) => optionOf(certificate, unusableLabel)),
  ];

  return (
    <Combobox
      label={t("panel.certificate.title")}
      options={options}
      value={chosen?.id ?? null}
      onChange={onChoose}
      renderOption={(certificate) => <CertificateCard certificate={certificate} />}
      searchPlaceholder={t("panel.certificate.search")}
      emptyMessage={t("panel.certificate.noMatch")}
      countLabel={(shown, total) => t("panel.certificate.matches", { shown, total })}
      listMaxHeight={listMaxHeight}
      alwaysGroupHeaders
      disabled={searching || absent !== undefined || disabled}
      defaultOpen={defaultOpen}
    >
      {closedBox(searching, absent, chosen, t)}
    </Combobox>
  );
}

function closedBox(
  searching: boolean,
  absent: "empty" | "failed" | undefined,
  chosen: Certificate | null,
  t: TFunction,
) {
  if (searching) {
    return (
      <>
        <span className="certificate-select__spinner">
          <SpinnerIcon size={16} />
        </span>
        <span className="rf-text-muted certificate-select__unchosen">
          {t("panel.certificate.loading")}
        </span>
      </>
    );
  }
  if (absent !== undefined) {
    return (
      <span className="rf-text-muted certificate-select__unchosen">
        {t(absent === "empty" ? "panel.certificate.empty.title" : "panel.certificate.failed.title")}
      </span>
    );
  }
  if (chosen === null) {
    return (
      <span className="rf-text-muted certificate-select__unchosen">
        {t("panel.certificate.chooseOne")}
      </span>
    );
  }
  return (
    <span className="certificate-select__text">
      <span className="certificate-select__chosen">{certificateHeadline(chosen)}</span>
      <span className="rf-body rf-text-muted certificate-select__ellipsis">
        {certificateCompactSubtitle(chosen, t)}
      </span>
    </span>
  );
}

function keywordsOf(certificate: Certificate, t: TFunction): string[] {
  return [
    certificate.holderName,
    certificate.entityName ?? "",
    certificate.organizationIdentifier ?? "",
    certificate.idNumber,
    certificate.issuer,
    ...certificate.stores.map((store) => storeLabel(store, t)),
    certificate.entityName === null ? t("panel.certificate.personalKeyword") : "",
  ];
}

function rowTooltip(certificate: Certificate, locale: string, t: TFunction): string {
  const issuer = t("panel.certificate.issuer", { issuer: certificate.issuer });
  const stores = certificate.stores;
  if (stores.length < 2) return issuer;
  const names = new Intl.ListFormat(locale, { type: "conjunction" }).format(
    stores.map((store) => storeLabel(store, t)),
  );
  return `${issuer} · ${t("panel.certificate.sameCertificateIn", { stores: names })}`;
}
