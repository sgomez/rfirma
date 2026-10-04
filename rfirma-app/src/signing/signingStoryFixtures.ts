//! Las firmas previas, los certificados y la rúbrica de ejemplo que comparten las historias de firma. Sin React.

import type { Certificate } from "./certificate";
import type {
  DocumentFinding,
  PreviousSignature,
  PreviousSignaturesReport,
  ValidityReason,
} from "./previousSignatures";
import type { Rubric } from "./rubric";

export function aSignature(overrides: Partial<PreviousSignature>): PreviousSignature {
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

export function aReport(
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

export const VALID_CLOSING = aSignature({ closesDocument: true });

export const EXPIRED = aSignature({
  name: "LUIS PEREZ",
  idNumber: "11111111H",
  organizationIdentifier: "B12345678",
  validity: "expired",
  validityReason: { kind: "certificateExpired", date: "2026-03-01", holder: null },
  signingDate: { kind: "stamped", at: "2026-02-10T09:00:00Z", tsa: "TSA de pruebas" },
});

export const NOT_ADMITTED = aSignature({
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

const IN_2020 = 1_579_046_400;
const IN_2030 = 1_893_456_000;
const IN_2099 = 4_070_908_800;

function aCertificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "0123456789abcdef",
    label: "FNMT-GEMELO",
    holderName: "LOVELACE BYRON ADA",
    stampedSigner: "LOVELACE BYRON ADA - NIF 0000****T",
    givenName: "Ada",
    surname: "Lovelace Byron",
    idNumber: "IDCES-00000000T",
    organizationIdentifier: null,
    entityName: null,
    issuer: "FNMT-RCM",
    certificateSerialNumber: "1234567890",
    stores: ["card"],
    status: { kind: "valid", notAfter: IN_2030 },
    remembered: false,
    ...overrides,
  };
}

export const PERSONAL_CERTIFICATE = aCertificate();

export const ENTITY_CERTIFICATE = aCertificate({
  id: "entity",
  holderName: "HOPPER BREWSTER GRACE",
  givenName: "Grace",
  surname: "Hopper Brewster",
  idNumber: "IDCES-00000001R",
  entityName: "Analytical Engines S.L.",
  organizationIdentifier: "VATES-B00000000",
  stores: ["firefox", "chrome"],
});

const EXPIRED_CERTIFICATE = aCertificate({
  id: "expired",
  holderName: "TURING ALAN",
  givenName: "Alan",
  surname: "Turing",
  idNumber: "IDCES-00000002W",
  status: { kind: "expired", notAfter: IN_2020 },
});

const NOT_YET_VALID_CERTIFICATE = aCertificate({
  id: "future",
  holderName: "HOLBERTON BETTY",
  givenName: "Betty",
  surname: "Holberton",
  idNumber: "IDCES-00000003A",
  status: { kind: "notYetValid", notBefore: IN_2099 },
});

export const STORY_CERTIFICATES: readonly Certificate[] = [
  PERSONAL_CERTIFICATE,
  ENTITY_CERTIFICATE,
  EXPIRED_CERTIFICATE,
  NOT_YET_VALID_CERTIFICATE,
];

export const STORY_RUBRIC: Rubric = {
  dataUrl:
    "data:image/svg+xml;utf8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='240' height='80'%3E%3Crect width='240' height='80' fill='white'/%3E%3Cpath d='M10 55 C40 5 70 80 110 30 S190 60 230 25' fill='none' stroke='black' stroke-width='3'/%3E%3C/svg%3E",
  width: 240,
  height: 80,
};
