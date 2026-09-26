import { useState } from "react";
import { useTranslation } from "react-i18next";
import { formatSignedAt } from "../App.signingOrder";
import { ChevronDownIcon, InfoIcon } from "../design-system/icons";
import type { PreviousSignature } from "./previousSignatures";

/**
 * El aviso de firmas previas: una línea plegada con «Firmarás junto a N firmas
 * anteriores» y, desplegada, una fila por firma con quién firmó y cuándo
 * (docs/design/panel-de-firma.md § El aviso de firmas previas). El llamador
 * solo lo monta con firmas, con una `key` por documento.
 */
export function PreviousSignaturesNotice({
  signatures,
}: {
  signatures: readonly PreviousSignature[];
}) {
  const { t, i18n } = useTranslation();
  const [expanded, setExpanded] = useState(signatures.length > 1);

  return (
    <div className="panel__co-signature">
      <button
        type="button"
        className="panel__co-signature-summary"
        aria-expanded={expanded}
        aria-label={t(expanded ? "panel.previousSignatures.hide" : "panel.previousSignatures.show")}
        onClick={() => setExpanded((current) => !current)}
      >
        <span className="panel__notice-icon">
          <InfoIcon />
        </span>
        <span className="rf-prose panel__co-signature-text">
          {t("panel.coSignature", { count: signatures.length })}
        </span>
        <span
          className={
            expanded
              ? "panel__co-signature-chevron panel__co-signature-chevron--open"
              : "panel__co-signature-chevron"
          }
        >
          <ChevronDownIcon size={14} strokeWidth={2} />
        </span>
      </button>
      {expanded && (
        <ul className="panel__previous-signatures-list">
          {signatures.map((signature) => (
            <li
              key={`${signature.certificateSerialNumber}-${signature.signingTime ?? ""}`}
              className="panel__previous-signatures-row"
            >
              <span className="rf-body">{signature.name}</span>
              <span className="rf-body rf-text-muted">
                {signature.signingTime === null
                  ? ""
                  : formatSignedAt(new Date(signature.signingTime), i18n.language)}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
