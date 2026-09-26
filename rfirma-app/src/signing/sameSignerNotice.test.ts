import { describe, expect, it } from "vitest";
import type { Certificate } from "./certificate";
import type { PreviousSignature } from "./previousSignatures";
import { sameSignerNotice } from "./sameSignerNotice";

function aCertificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "0123456789abcdef0123456789abcdef",
    label: "Firma",
    holderName: "Ada Lovelace Byron",
    stampedSigner: "Ada Lovelace Byron",
    givenName: "Ada",
    surname: "Lovelace Byron",
    idNumber: "99999999R",
    organizationIdentifier: null,
    issuer: "AC FNMT Usuarios",
    certificateSerialNumber: "1234567890",
    store: "card",
    status: { kind: "valid", notAfter: 1_894_752_000 },
    remembered: false,
    ...overrides,
  };
}

function aSignature(overrides: Partial<PreviousSignature> = {}): PreviousSignature {
  return {
    name: "Ada Lovelace Byron",
    idNumber: "99999999R",
    organizationIdentifier: null,
    issuer: "AC FNMT Usuarios",
    certificateSerialNumber: "1234567890",
    signingTime: "2024-01-01T10:00:00Z",
    ...overrides,
  };
}

describe("sameSignerNotice", () => {
  it("is nothing without a certificate chosen", () => {
    expect(sameSignerNotice(null, [aSignature()])).toBe(null);
  });

  it("is nothing without any previous signature", () => {
    expect(sameSignerNotice(aCertificate(), [])).toBe(null);
  });

  it("says the same certificate when the issuer and the serial number match", () => {
    expect(sameSignerNotice(aCertificate(), [aSignature()])).toBe("sameCertificate");
  });

  it("says another certificate of yours with a renewed certificate: same NIF and entity, other serial", () => {
    const notice = sameSignerNotice(aCertificate(), [
      aSignature({ certificateSerialNumber: "9999999999" }),
    ]);

    expect(notice).toBe("otherCertificate");
  });

  it("stays silent for a representative certificate against a previous signature as a private person", () => {
    const notice = sameSignerNotice(aCertificate({ organizationIdentifier: "VATES-A00000000" }), [
      aSignature({ organizationIdentifier: null }),
    ]);

    expect(notice).toBe(null);
  });

  it("stays silent for two different represented entities", () => {
    const notice = sameSignerNotice(aCertificate({ organizationIdentifier: "VATES-A00000000" }), [
      aSignature({ organizationIdentifier: "VATES-B00000000" }),
    ]);

    expect(notice).toBe(null);
  });
});
