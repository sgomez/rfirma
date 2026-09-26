import { act } from "@testing-library/react";
import type { Mock } from "vitest";
import { vi } from "vitest";
import type { Certificate } from "../signing/certificate";
import type { Errand, ErrandStage, SiteDocument, SiteErrandPort } from "./errand";
import { noErrand } from "./errand";

/** Los dobles y auxiliares que comparten las pruebas de `SedeWindow` (TD-63). */

export function certificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "handle-1",
    label: "FNMT",
    holderName: "ADA LOVELACE BYRON",
    givenName: "ADA",
    surname: "LOVELACE BYRON",
    idNumber: "99999999R",
    issuer: "FNMT-RCM",
    store: "installed",
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
    consent: vi.fn(),
    confirmSignatures: vi.fn(),
    markArea: vi.fn(),
    cancel: vi.fn(),
    close: vi.fn(),
    lookAgain: vi.fn(),
    installCertificate: vi.fn(),
    installLocalCa: vi.fn(),
    dismissWarning: vi.fn(),
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

/** El documento del artboard, el mismo que se enseña al consentir. */
export const signedDocument: SiteDocument = {
  title: "Solicitud de subvención 2026",
  pages: 27,
  sizeBytes: 2_400_000,
  round: { kind: "sign" },
  hasUnregisteredSignatures: false,
};

/** Deja pasar el tiempo con los relojes falsos, y deja que React repinte. */
export async function elapse(ms: number) {
  await act(async () => {
    vi.advanceTimersByTime(ms);
  });
}
