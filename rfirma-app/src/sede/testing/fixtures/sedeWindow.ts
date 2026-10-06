//! Los dobles de `SiteErrandPort` y los momentos de ejemplo que comparten las pruebas de `SedeWindow`.

import type { Mock } from "storybook/test";
import { fn } from "storybook/test";
import type { Certificate } from "../../../signing/certificate";
import { NO_PREVIOUS_SIGNATURES } from "../../../signing/previousSignatures";
import type { Errand, ErrandStage, SiteDocument, SiteErrandPort } from "../../errand";
import { noErrand } from "../../errand";

/** Los dobles y auxiliares que comparten las pruebas de `SedeWindow`. */

export function certificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "handle-1",
    label: "FNMT",
    holderName: "ADA LOVELACE BYRON",
    stampedSigner: "ADA LOVELACE BYRON",
    givenName: "ADA",
    surname: "LOVELACE BYRON",
    idNumber: "99999999R",
    organizationIdentifier: null,
    entityName: null,
    issuer: "FNMT-RCM",
    certificateSerialNumber: "1234567890",
    stores: ["installed"],
    status: { kind: "valid", notAfter: 4_102_444_800 },
    remembered: false,
    ...overrides,
  };
}

/** Un puerto que emite el momento que se le pida, y anota lo que se le llama. */
export function scriptedErrand(stage: ErrandStage, errand: Partial<Errand> = {}) {
  const calls: Record<
    | "consent"
    | "confirmSignatures"
    | "markArea"
    | "cancel"
    | "close"
    | "lookAgain"
    | "installCertificate"
    | "installLocalCa"
    | "dismissWarning",
    Mock
  > = {
    consent: fn(),
    confirmSignatures: fn(),
    markArea: fn(),
    cancel: fn(),
    close: fn(),
    lookAgain: fn(),
    installCertificate: fn(),
    installLocalCa: fn(),
    dismissWarning: fn(),
  };
  const port: SiteErrandPort = {
    ...noErrand(),
    watch: (onChange) => {
      onChange({ origin: "sede.ejemplo.gob.es", operation: "sign", stage, ...errand });
      return () => {};
    },
    consent: async (id) => calls.consent(id),
    confirmSignatures: async () => calls.confirmSignatures(),
    markArea: async (area) => calls.markArea(area),
    cancel: async () => calls.cancel(),
    close: async () => calls.close(),
    lookAgain: async () => calls.lookAgain(),
    installCertificate: async () => calls.installCertificate(),
    installLocalCa: async () => calls.installLocalCa(),
    dismissWarning: async () => calls.dismissWarning(),
  };
  return { port, calls };
}

/** El trámite de una historia, para montar su estado en un test de comportamiento. */
export function errandOf(story: { args: { errand?: Errand } }): Errand {
  if (story.args.errand === undefined) throw new Error("the story carries no errand");
  return story.args.errand;
}

/** Un puerto guionizado que arranca en el trámite de una historia. */
export function scriptedFrom(story: { args: { errand?: Errand } }) {
  const errand = errandOf(story);
  return scriptedErrand(errand.stage, errand);
}

/** El documento del artboard, el mismo que se enseña al consentir. */
export const signedDocument: SiteDocument = {
  title: "Solicitud de subvención 2026",
  pages: 27,
  sizeBytes: 2_400_000,
  round: { kind: "sign" },
  previousSignatures: NO_PREVIOUS_SIGNATURES,
};

type ConsentStage = Extract<ErrandStage, { kind: "consent" }>;

/** El consentimiento de una firma de PDF con un solo certificado; `overrides` cambia lo que cuente la variante. */
export function consentStage(overrides: Partial<ConsentStage> = {}): ConsentStage {
  return {
    kind: "consent",
    document: signedDocument,
    signs: null,
    signing: "pdf",
    items: null,
    certificates: [certificate()],
    narrowed: false,
    sha1Allowed: false,
    sha1ToAllow: false,
    ...overrides,
  };
}
