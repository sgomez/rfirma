import { describe, expect, it } from "vitest";
import type { Certificate } from "./certificate";
import { refusalFor } from "./failure";

function withStatus(status: Certificate["status"]): Certificate {
  return {
    label: "Firma",
    holderName: "Ada Lovelace Byron",
    idNumber: "99999999R",
    issuer: "AC FNMT Usuarios",
    status,
  };
}

// Grada A: es una decisión sobre datos, sin token y sin red.
describe("refusalFor", () => {
  it("lets a certificate in force through", () => {
    expect(refusalFor(withStatus({ kind: "valid" }))).toBeNull();
  });

  it("refuses an expired certificate, keeping the date in the raw detail", () => {
    const refusal = refusalFor(withStatus({ kind: "expired", notAfter: 1_767_225_600 }));

    expect(refusal?.situation).toBe("certificateExpired");
    expect(refusal?.detail).toContain("1767225600");
  });

  it("refuses a revoked certificate, keeping the reason untranslated", () => {
    const refusal = refusalFor(withStatus({ kind: "revoked", reason: "keyCompromise" }));

    expect(refusal?.situation).toBe("certificateRevoked");
    expect(refusal?.detail).toContain("keyCompromise");
  });

  it("refuses one that is not in force yet, and one it cannot read", () => {
    expect(refusalFor(withStatus({ kind: "notYetValid" }))?.situation).toBe(
      "certificateNotYetValid",
    );
    expect(refusalFor(withStatus({ kind: "unreadable" }))?.situation).toBe("certificateUnreadable");
  });

  it("refuses when no certificate was chosen at all", () => {
    expect(refusalFor(null)?.situation).toBe("certificateNotFound");
  });
});
