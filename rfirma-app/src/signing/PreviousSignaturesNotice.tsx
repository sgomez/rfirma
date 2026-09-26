import type { TFunction } from "i18next";
import type { ReactNode } from "react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { formatSignedAt } from "../App.signingOrder";
import {
  AlertIcon,
  CheckCircleIcon,
  ChevronDownIcon,
  CrossCircleIcon,
  DottedCircleIcon,
  InfoIcon,
  PersonIcon,
} from "../design-system/icons";
import type { Certificate } from "./certificate";
import type {
  PreviousSignature,
  PreviousSignaturesReport,
  SignatureStatus,
  Tone,
} from "./previousSignatures";
import { sameSignerNotice } from "./sameSignerNotice";

/**
 * El aviso de firmas previas: una línea plegada con «Firmarás junto a N firmas
 * anteriores» y los avisos del backend, y desplegada, una fila por firma con
 * su veredicto y motivo, y una franja al pie con «ya lo firmaste tú» si el
 * certificado elegido coincide (docs/design/panel-de-firma.md § El aviso de
 * firmas previas). El llamador solo lo monta con firmas, con una `key` por
 * documento.
 */
export function PreviousSignaturesNotice({
  report,
  certificate,
}: {
  report: PreviousSignaturesReport;
  certificate: Certificate | null;
}) {
  const { t, i18n } = useTranslation();
  const { signatures, warningCount, tone, changedAfterLastSignature } = report;
  const [expanded, setExpanded] = useState(signatures.length > 1 || warningCount > 0);
  const notice = sameSignerNotice(certificate, signatures);
  const hasBroken = signatures.some((signature) => signature.status === "broken");

  return (
    <div className={`panel__co-signature panel__co-signature--${tone}`}>
      <button
        type="button"
        className="panel__co-signature-summary"
        aria-expanded={expanded}
        aria-label={t(expanded ? "panel.previousSignatures.hide" : "panel.previousSignatures.show")}
        onClick={() => setExpanded((current) => !current)}
      >
        <span className="panel__notice-icon">{summaryIcon(tone, hasBroken)}</span>
        <span className="rf-prose panel__co-signature-text">
          <span className="panel__co-signature-count">
            {t("panel.coSignature", { count: signatures.length })}
          </span>
          {warningCount > 0 && (
            <span className="panel__co-signature-warnings">
              {t("panel.previousSignatures.warnings", { count: warningCount })}
            </span>
          )}
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
          {signatures.map((signature, index) => (
            <PreviousSignatureRow
              key={`${signature.certificateSerialNumber}-${signature.signingTime ?? ""}`}
              signature={signature}
              locale={i18n.language}
              t={t}
              changed={index === signatures.length - 1 && changedAfterLastSignature}
            />
          ))}
        </ul>
      )}
      {notice !== null && (
        <div className="panel__co-signature-footer">
          <span className="panel__notice-icon">
            <PersonIcon />
          </span>
          <span className="rf-body">
            {notice === "sameCertificate"
              ? t("panel.previousSignatures.sameCertificate")
              : t("panel.previousSignatures.otherCertificate")}
          </span>
        </div>
      )}
    </div>
  );
}

function PreviousSignatureRow({
  signature,
  locale,
  t,
  changed,
}: {
  signature: PreviousSignature;
  locale: string;
  t: TFunction;
  changed: boolean;
}) {
  const bold = signature.status !== "valid" && signature.status !== "notFullyChecked";
  const reason = reasonLabel(t, signature.status, signature.reason);
  return (
    <li className="panel__previous-signatures-row">
      <div className="panel__previous-signatures-row-top">
        <span className="rf-body">{signature.name}</span>
        <span
          className={`panel__previous-signatures-verdict${bold ? " panel__previous-signatures-verdict--strong" : ""}`}
        >
          {statusIcon(signature.status)}
          <span className="rf-body">{statusLabel(t, signature.status)}</span>
        </span>
      </div>
      <div className="rf-body rf-text-muted panel__previous-signatures-row-meta">
        {signature.signingTime === null
          ? ""
          : formatSignedAt(new Date(signature.signingTime), locale)}
      </div>
      {reason !== null && (
        <div className="rf-body rf-text-muted panel__previous-signatures-row-meta">{reason}</div>
      )}
      {changed && (
        <div className="panel__previous-signatures-changed">
          <AlertIcon size={14} />
          <span className="rf-body">{t("panel.previousSignatures.changedAfterSignature")}</span>
        </div>
      )}
    </li>
  );
}

function summaryIcon(tone: Tone, hasBroken: boolean): ReactNode {
  switch (tone) {
    case "information":
      return <InfoIcon />;
    case "indeterminate":
      return <DottedCircleIcon />;
    case "attention":
      return hasBroken ? <CrossCircleIcon size={20} /> : <AlertIcon />;
  }
}

function statusIcon(status: SignatureStatus): ReactNode {
  switch (status) {
    case "valid":
      return <CheckCircleIcon size={16} />;
    case "certificateExpired":
    case "certificateNotYetValid":
    case "unverifiable":
      return <AlertIcon size={16} />;
    case "broken":
      return <CrossCircleIcon size={16} />;
    case "notFullyChecked":
      return <DottedCircleIcon size={16} />;
  }
}

function statusLabel(t: TFunction, status: SignatureStatus): string {
  switch (status) {
    case "valid":
      return t("panel.previousSignatures.status.valid");
    case "certificateExpired":
      return t("panel.previousSignatures.status.certificateExpired");
    case "certificateNotYetValid":
      return t("panel.previousSignatures.status.certificateNotYetValid");
    case "broken":
      return t("panel.previousSignatures.status.broken");
    case "unverifiable":
      return t("panel.previousSignatures.status.unverifiable");
    case "notFullyChecked":
      return t("panel.previousSignatures.status.notFullyChecked");
  }
}

/** El motivo bajo el veredicto, o `null` para una firma válida. */
function reasonLabel(t: TFunction, status: SignatureStatus, reason: string | null): string | null {
  switch (status) {
    case "valid":
      return null;
    case "certificateExpired":
      return t("panel.previousSignatures.reason.certificateExpired");
    case "certificateNotYetValid":
      return t("panel.previousSignatures.reason.certificateNotYetValid");
    case "unverifiable":
      return t("panel.previousSignatures.reason.unverifiable");
    case "notFullyChecked":
      return t("panel.previousSignatures.reason.notFullyChecked");
    case "broken":
      return brokenReasonLabel(t, reason);
  }
}

function brokenReasonLabel(t: TFunction, reason: string | null): string {
  switch (reason) {
    case "NO_MATCH_DATA":
      return t("panel.previousSignatures.reason.brokenDataMismatch");
    case "CERTIFIED_SIGN_REVISION":
      return t("panel.previousSignatures.reason.brokenCertifiedRevision");
    default:
      return t("panel.previousSignatures.reason.brokenCorrupted");
  }
}
