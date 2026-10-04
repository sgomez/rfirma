//! El texto del motivo por el que una firma está caducada o no es válida, y el de cada hallazgo del documento.

import type { TFunction } from "i18next";
import type { DocumentFinding, ValidityReason } from "./previousSignatures";

function formatDate(instant: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, { dateStyle: "long" }).format(new Date(instant));
}

export function validityReasonText(t: TFunction, reason: ValidityReason, locale: string): string {
  switch (reason.kind) {
    case "certificateExpired": {
      const date = formatDate(reason.date, locale);
      return reason.holder === null
        ? t("signatureReason.certificateExpired", { date })
        : t("signatureReason.certificateExpiredHolder", {
            date,
            holder: reason.holder,
          });
    }
    case "modifiedAfterSigning":
      return t("signatureReason.modifiedAfterSigning");
    case "damaged":
      return t("signatureReason.damaged");
    case "certificateNotYetValid":
      return t("signatureReason.certificateNotYetValid", {
        date: formatDate(reason.date, locale),
      });
    case "unknownSignatureType":
      return t("signatureReason.unknownSignatureType");
    case "unsupportedAlgorithm":
      return t("signatureReason.unsupportedAlgorithm");
    case "cosignNotAdmitted":
      return reason.closedBy === null
        ? t("signatureReason.cosignNotAdmittedUnnamed")
        : t("signatureReason.cosignNotAdmitted", { name: reason.closedBy });
  }
}

export function findingText(t: TFunction, finding: DocumentFinding): string {
  switch (finding) {
    case "modifiedAfterLastSignature":
      return t("documentFinding.modifiedAfterLastSignature");
    case "formFilledAfterSigning":
      return t("documentFinding.formFilledAfterSigning");
    case "contentAddedOnTop":
      return t("documentFinding.contentAddedOnTop");
  }
}
