import { describe, expect, it } from "vitest";
import { hasMenuAttention, type SignalRow } from "./status";

function row(overrides: Partial<SignalRow>): SignalRow {
  return {
    signal: "version",
    value: "",
    verdict: "correct",
    action: null,
    detail: null,
    candidates: null,
    restartFirefoxNotice: false,
    ...overrides,
  };
}

describe("hasMenuAttention", () => {
  it("stays off when every signal is correct", () => {
    expect(
      hasMenuAttention([
        row({ signal: "version", verdict: "correct", value: "0.4.1" }),
        row({ signal: "siteSignature", verdict: "correct", value: "rFirma" }),
        row({ signal: "localCaCertificate", verdict: "correct", value: "3/3" }),
        row({ signal: "userCertificates", verdict: "correct", value: "2" }),
      ]),
    ).toBe(false);
  });

  it("lights up when the local CA certificate is absent (0 trusted)", () => {
    expect(
      hasMenuAttention([row({ signal: "localCaCertificate", verdict: "incorrect", value: "0/3" })]),
    ).toBe(true);
  });

  it("lights up when the local CA certificate is half done", () => {
    expect(
      hasMenuAttention([row({ signal: "localCaCertificate", verdict: "attention", value: "2/3" })]),
    ).toBe(true);
  });

  it("lights up when nobody handles the sites (Sin configurar)", () => {
    expect(
      hasMenuAttention([row({ signal: "siteSignature", verdict: "attention", value: "" })]),
    ).toBe(true);
  });

  it("stays off when the sites open AutoFirma, even though the row says Atención", () => {
    expect(
      hasMenuAttention([
        row({ signal: "siteSignature", verdict: "attention", value: "AutoFirma" }),
      ]),
    ).toBe(false);
  });

  it("stays off when the site signature does not apply", () => {
    expect(
      hasMenuAttention([row({ signal: "siteSignature", verdict: "notApplicable", value: "" })]),
    ).toBe(false);
  });

  it("stays off when there are no user certificates", () => {
    expect(
      hasMenuAttention([row({ signal: "userCertificates", verdict: "attention", value: "0" })]),
    ).toBe(false);
  });

  it("stays off when a new version is available", () => {
    expect(
      hasMenuAttention([row({ signal: "version", verdict: "attention", value: "0.5.0" })]),
    ).toBe(false);
  });
});
