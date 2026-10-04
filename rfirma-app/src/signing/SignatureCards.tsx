//! Los hallazgos del documento y una ficha por firma con su validez, las mismas en el resumen y en el diálogo «Ver firmas».

import type { TFunction } from "i18next";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../design-system/Badge";
import { AlertIcon, CheckCircleIcon, CrossCircleIcon } from "../design-system/icons";
import { Row } from "../design-system/Row";
import type {
  DocumentFinding,
  PreviousSignature,
  SigningDate,
  Validity,
} from "./previousSignatures";
import { findingText, validityReasonText } from "./validityReasons";
import "./SignatureCards.css";

interface SignatureCardsProps {
  signatures: readonly PreviousSignature[];
  findings: readonly DocumentFinding[];
  /** Es el resumen de lo que se acaba de firmar: la última ficha es la propia y el cambio es anterior a ella. */
  justSigned?: boolean;
}

/** Los hallazgos del documento, encima, y las fichas de sus firmas apiladas. */
export function SignatureCards({ signatures, findings, justSigned = false }: SignatureCardsProps) {
  const { t, i18n } = useTranslation();
  const locale = i18n.language;
  return (
    <>
      {findings.length > 0 && (
        <ul className="signature-cards__findings">
          {findings.map((finding) => (
            <li className="signature-cards__finding" key={finding}>
              <span className="signature-cards__finding-icon">
                <CrossCircleIcon size={15} />
              </span>
              <span>{findingLabel(t, finding, justSigned)}</span>
            </li>
          ))}
        </ul>
      )}
      <ol className="signature-cards">
        {signatures.map((signature, index) => (
          <li
            className="rf-card signature-cards__card"
            // biome-ignore lint/suspicious/noArrayIndexKey: el orden es la identidad de la firma.
            key={index}
          >
            <CardHead
              label={t("panel.signed.signature", { number: index + 1 })}
              signature={signature}
              isNew={justSigned && index === signatures.length - 1}
            />
            <SignatureRows signature={signature} locale={locale} />
            <Countersignatures
              signature={signature}
              numbering={String(index + 1)}
              locale={locale}
            />
          </li>
        ))}
      </ol>
    </>
  );
}

function findingLabel(t: TFunction, finding: DocumentFinding, justSigned: boolean): string {
  return justSigned && finding === "modifiedAfterLastSignature"
    ? t("documentFinding.modifiedBeforeYourSignature")
    : findingText(t, finding);
}

function CardHead({
  label,
  signature,
  isNew,
}: {
  label: string;
  signature: PreviousSignature;
  isNew: boolean;
}) {
  const { t } = useTranslation();
  return (
    <Row className="signature-cards__head">
      <span className="rf-label signature-cards__label">{label}</span>
      {isNew && <Badge variant="primary">{t("panel.signed.new")}</Badge>}
      <span
        className={`signature-cards__validity signature-cards__validity--${signature.validity}`}
      >
        {validityIcon(signature.validity)}
        <span className="rf-body">{validityLabel(t, signature.validity)}</span>
      </span>
    </Row>
  );
}

function validityIcon(validity: Validity): ReactNode {
  switch (validity) {
    case "valid":
      return <CheckCircleIcon size={14} />;
    case "expired":
      return <AlertIcon size={14} />;
    case "invalid":
      return <CrossCircleIcon size={14} />;
  }
}

function validityLabel(t: TFunction, validity: Validity): string {
  switch (validity) {
    case "valid":
      return t("panel.signed.validity.valid");
    case "expired":
      return t("panel.signed.validity.expired");
    case "invalid":
      return t("panel.signed.validity.invalid");
  }
}

function Countersignatures({
  signature,
  numbering,
  locale,
}: {
  signature: PreviousSignature;
  numbering: string;
  locale: string;
}) {
  const { t } = useTranslation();
  return signature.countersignatures.map((countersignature, index) => {
    const path = `${numbering}.${index + 1}`;
    return (
      // biome-ignore lint/suspicious/noArrayIndexKey: el orden es la identidad de la firma.
      <div className="signature-cards__countersignature" key={index}>
        <CardHead
          label={t("panel.signed.countersignature", { number: path })}
          signature={countersignature}
          isNew={false}
        />
        <SignatureRows signature={countersignature} locale={locale} />
        <Countersignatures signature={countersignature} numbering={path} locale={locale} />
      </div>
    );
  });
}

function SignatureRows({ signature, locale }: { signature: PreviousSignature; locale: string }) {
  const { t } = useTranslation();
  const rows: [string, string, boolean][] = [
    [t("panel.signed.field.signer"), signerOf(signature), true],
    [t("panel.signed.field.onBehalfOf"), signature.organizationIdentifier ?? "", false],
    [t("panel.signed.field.issuer"), signature.issuer, false],
    ...dateRows(t, signature.signingDate, locale),
  ];
  const reason =
    signature.validityReason === null
      ? ""
      : validityReasonText(t, signature.validityReason, locale);

  return (
    <>
      {rows
        .filter(([, value]) => value !== "")
        .map(([label, value, strong]) => (
          <SignatureField key={label} label={label} value={value} strong={strong} />
        ))}
      {signature.closesDocument && (
        <span className="rf-body signature-cards__closes">{t("panel.signed.closesDocument")}</span>
      )}
      {reason !== "" && (
        <SignatureField label={t("panel.signed.field.reason")} value={reason} strong={false} />
      )}
    </>
  );
}

function SignatureField({
  label,
  value,
  strong,
}: {
  label: string;
  value: string;
  strong: boolean;
}) {
  return (
    <Row className="signature-cards__field">
      <span className="rf-body rf-text-muted signature-cards__field-label">{label}</span>
      <span
        className={`rf-body signature-cards__field-value${strong ? " signature-cards__field-value--signer" : ""}`}
      >
        {value}
      </span>
    </Row>
  );
}

function dateRows(
  t: TFunction,
  date: SigningDate | null,
  locale: string,
): [string, string, boolean][] {
  if (date === null) return [];
  const at = formatCardDate(date.at, locale);
  return date.kind === "declared"
    ? [[t("panel.signed.field.date"), at, false]]
    : [[t("panel.signed.field.sealed"), `${at} · ${date.tsa}`, false]];
}

function formatCardDate(instant: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(
    new Date(instant),
  );
}

function signerOf(signature: PreviousSignature): string {
  return signature.idNumber === "" ? signature.name : `${signature.name} (${signature.idNumber})`;
}
