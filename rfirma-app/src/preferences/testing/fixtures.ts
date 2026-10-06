//! Los ajustes y los certificados instalados de ejemplo, compartidos por las historias y las pruebas de Preferencias.

import type { Certificate } from "../../signing/certificate";
import type { Preferences } from "../preferences";

export const defaults: Preferences = {
  theme: "system",
  destination: "Documentos",
  destinationMode: "next_to_the_original",
  offersOriginalFolder: false,
  rememberVisibleSignature: true,
  rememberActivity: true,
  notifyNewVersion: true,
  setupWizardSeen: false,
  consentCountdown: true,
  honourAutomaticSelection: false,
  allowSha1: false,
};

/** `2030-01-15T00:00:00Z`, en segundos desde la época. */
const IN_2030 = 1_894_752_000;

/** `2020-01-15T00:00:00Z`, en segundos desde la época. */
export const IN_2020 = 1_579_046_400;

export function anInstalledCertificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "0123456789abcdef",
    label: "FNMT-GEMELO",
    holderName: "Ada Lovelace Byron",
    stampedSigner: "Ada Lovelace Byron",
    givenName: "Ada",
    surname: "Lovelace Byron",
    idNumber: "IDCES-00000000T",
    organizationIdentifier: null,
    entityName: null,
    issuer: "FNMT-RCM",
    certificateSerialNumber: "1234567890",
    stores: ["installed"],
    status: { kind: "valid", notAfter: IN_2030 },
    remembered: false,
    ...overrides,
  };
}
