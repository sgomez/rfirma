import { describe, expect, it } from "vitest";
import type { Certificate } from "./certificate";
import { groupCertificates } from "./certificate";

function aCertificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "0123456789abcdef0123456789abcdef",
    label: "Firma",
    holderName: "Ada Lovelace Byron",
    idNumber: "99999999R",
    issuer: "AC FNMT Usuarios",
    store: "card",
    status: { kind: "valid" },
    remembered: false,
    ...overrides,
  };
}

describe("groupCertificates", () => {
  it("puts the usable ones in the available group and the rest in the unusable one", () => {
    const valid = aCertificate({ id: "valid" });
    const expired = aCertificate({ id: "expired", status: { kind: "expired", notAfter: 0 } });

    const groups = groupCertificates([expired, valid]);

    expect(groups.available).toEqual([valid]);
    expect(groups.unusable).toEqual([expired]);
  });

  /** Caducado, aún no válido y no leído son motivos distintos, pero caen en el
   * mismo grupo: la agrupación es «se puede firmar o no», no el motivo. */
  it("groups every unusable reason together, whatever it is", () => {
    const expired = aCertificate({ id: "expired", status: { kind: "expired", notAfter: 0 } });
    const notYetValid = aCertificate({
      id: "notYetValid",
      status: { kind: "notYetValid", notBefore: 0 },
    });
    const unreadable = aCertificate({
      id: "unreadable",
      status: { kind: "unreadable", detail: "DER roto" },
    });

    const groups = groupCertificates([expired, notYetValid, unreadable]);

    expect(groups.available).toEqual([]);
    expect(groups.unusable).toHaveLength(3);
  });

  /** Un estado que hoy no emite nadie (#194): la agrupación no cambia el día
   * que empiece a emitirse. */
  it("files a revoked certificate under unusable too", () => {
    const revoked = aCertificate({ id: "revoked", status: { kind: "revoked", reason: "" } });

    const groups = groupCertificates([revoked]);

    expect(groups.unusable).toEqual([revoked]);
  });

  it("sorts each group alphabetically by holder, in Spanish, accents and «ñ» included", () => {
    const nino = aCertificate({ id: "a", holderName: "Niño Pérez" });
    const zutano = aCertificate({ id: "b", holderName: "Zutano García" });
    const alvaro = aCertificate({ id: "c", holderName: "Álvaro Núñez" });

    const groups = groupCertificates([zutano, nino, alvaro]);

    expect(groups.available.map((c) => c.holderName)).toEqual([
      "Álvaro Núñez",
      "Niño Pérez",
      "Zutano García",
    ]);
  });

  it("breaks a tie between same-holder certificates by store", () => {
    const chrome = aCertificate({ id: "chrome", store: "chrome" });
    const card = aCertificate({ id: "card", store: "card" });
    const firefox = aCertificate({ id: "firefox", store: "firefox" });

    const groups = groupCertificates([chrome, firefox, card]);

    expect(groups.available.map((c) => c.store)).toEqual(["card", "chrome", "firefox"]);
  });

  it("returns two empty groups for an empty list", () => {
    expect(groupCertificates([])).toEqual({ available: [], unusable: [] });
  });
});
