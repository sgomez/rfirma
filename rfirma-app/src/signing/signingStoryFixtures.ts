//! Las firmas previas de ejemplo que comparten las historias de los diálogos de firma. Sin React.

import type {
  DocumentFinding,
  PreviousSignature,
  PreviousSignaturesReport,
  ValidityReason,
} from "./previousSignatures";

function aSignature(overrides: Partial<PreviousSignature>): PreviousSignature {
  return {
    name: "ADA LOVELACE",
    idNumber: "00000000T",
    organizationIdentifier: null,
    issuer: "AC FNMT Usuarios",
    certificateSerialNumber: "1",
    signingTime: "2026-09-14T10:32:05Z",
    validity: "valid",
    validityReason: null,
    signingDate: { kind: "declared", at: "2026-09-14T10:32:05Z" },
    closesDocument: false,
    countersignatures: [],
    ...overrides,
  };
}

function aReport(
  signatures: readonly PreviousSignature[],
  findings: readonly DocumentFinding[] = [],
): PreviousSignaturesReport {
  return {
    signatures,
    warningCount: signatures.filter((each) => each.validity !== "valid").length + findings.length,
    tone: "attention",
    changedAfterLastSignature: findings.length > 0,
    findings,
  };
}

const VALID_CLOSING = aSignature({ closesDocument: true });

const EXPIRED = aSignature({
  name: "LUIS PEREZ",
  idNumber: "11111111H",
  organizationIdentifier: "B12345678",
  validity: "expired",
  validityReason: { kind: "certificateExpired", date: "2026-03-01", holder: null },
  signingDate: { kind: "stamped", at: "2026-02-10T09:00:00Z", tsa: "TSA de pruebas" },
});

const NOT_ADMITTED = aSignature({
  name: "MARTA RUIZ",
  idNumber: "22222222J",
  issuer: "AC Camerfirma",
  validity: "invalid",
  validityReason: { kind: "cosignNotAdmitted", closedBy: "ANA LOPEZ GARCIA" },
});

const UNRECOGNIZED = aSignature({
  name: "NOTARIA XYZ",
  idNumber: "33333333P",
  validity: "invalid",
  validityReason: { kind: "unknownSignatureType" },
});

const EVERY_REASON: readonly ValidityReason[] = [
  { kind: "certificateExpired", date: "2026-03-01", holder: "LUIS PEREZ" },
  { kind: "modifiedAfterSigning" },
  { kind: "damaged" },
  { kind: "certificateNotYetValid", date: "2027-01-01" },
  { kind: "unknownSignatureType" },
  { kind: "unsupportedAlgorithm" },
  { kind: "cosignNotAdmitted", closedBy: "ANA LOPEZ GARCIA" },
];

const EVERY_FINDING: readonly DocumentFinding[] = [
  "modifiedAfterLastSignature",
  "formFilledAfterSigning",
  "contentAddedOnTop",
];

export const MIXED_REPORT = aReport(
  [VALID_CLOSING, EXPIRED, NOT_ADMITTED],
  ["modifiedAfterLastSignature"],
);

export const ALL_VALID_REPORT = aReport([
  VALID_CLOSING,
  aSignature({ name: "GRACE HOPPER", idNumber: "44444444A" }),
]);

export const EXPIRED_ONLY_REPORT = aReport([VALID_CLOSING, EXPIRED]);

export const UNRECOGNIZED_REPORT: PreviousSignaturesReport = {
  ...aReport([VALID_CLOSING, UNRECOGNIZED]),
  format: "unrecognized",
};

export const EXTREME_REPORT = aReport(
  EVERY_REASON.map((reason, index) =>
    aSignature({
      name: `FIRMANTE ${index + 1}`,
      idNumber: `0000000${index}T`,
      validity: reason.kind === "certificateExpired" ? "expired" : "invalid",
      validityReason: reason,
    }),
  ),
  EVERY_FINDING,
);
