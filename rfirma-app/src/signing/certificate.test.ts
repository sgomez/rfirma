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
    const alvaro = aCertificate({ id: "a", holderName: "Álvaro Núñez" });
    const zutano = aCertificate({ id: "b", holderName: "Zutano García" });
    const alfonso = aCertificate({ id: "c", holderName: "Alfonso Ruiz" });

    const groups = groupCertificates([zutano, alvaro, alfonso]);

    // «Álvaro» y «Alfonso» solo se separan al comparar con `sensitivity:
    // "base"`, que iguala la tilde con la letra sin tilde y deja decidir a la
    // «l»/«v» siguiente: sin el `"es"` del `Intl.Collator` (o con `en`, `de`,
    // sin locale) esta terna ya da otro orden.
    expect(groups.available.map((c) => c.holderName)).toEqual([
      "Alfonso Ruiz",
      "Álvaro Núñez",
      "Zutano García",
    ]);
  });

  /** En la colación de `es`, la «ñ» es letra propia y ordena **después de
   * toda la n**: en `en` es una n con tilde y el orden se invierte. Este par
   * falla el día que alguien quite el `"es"` del `Intl.Collator`, que es
   * justo lo que este criterio protege (#197, TD del #194). */
  it("orders «ñ» after every plain «n», the Spanish way", () => {
    const penz = aCertificate({ id: "a", holderName: "Penz Ruiz" });
    const pena = aCertificate({ id: "b", holderName: "Peña Ruiz" });

    const groups = groupCertificates([pena, penz]);

    expect(groups.available.map((c) => c.holderName)).toEqual(["Penz Ruiz", "Peña Ruiz"]);
  });

  /** `sensitivity: "base"` iguala dos titulares que solo se distinguen por la
   * tilde: decide entonces el desempate por almacén, no el nombre. Con el
   * `sensitivity` por defecto ("variant") el resultado sería otro. */
  it("treats holders that differ only by an accent as equal, deciding by store", () => {
    const angel = aCertificate({ id: "chrome", holderName: "Ángel Ruiz", store: "chrome" });
    const angelPlain = aCertificate({ id: "card", holderName: "Angel Ruiz", store: "card" });

    const groups = groupCertificates([angel, angelPlain]);

    expect(groups.available.map((c) => c.store)).toEqual(["card", "chrome"]);
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
