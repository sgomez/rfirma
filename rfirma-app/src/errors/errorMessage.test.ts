import { describe, expect, it } from "vitest";
import { createI18n } from "../i18n/i18n";
import { LANGUAGES } from "../i18n/languages";
import { type ErrorSituation, errorText, MESSAGE_OF } from "./errorMessage";

const { t } = createI18n("es");

const SITUATIONS = Object.keys(MESSAGE_OF) as ErrorSituation[];

describe("the message of each error situation", () => {
  it("tells an expired certificate to renew it", () => {
    expect(errorText("certificateExpired", t)).toEqual({
      title: "Tu certificado ha caducado",
      body: "Renuévalo con su emisora.",
    });
  });

  it("puts the warning of a certified document in its title", () => {
    expect(errorText("documentCertified", t).title).toBe(
      "Firmarlo invalidaría la certificación del autor",
    );
  });

  it("tells a revoked certificate in one line", () => {
    expect(errorText("certificateRevoked", t)).toEqual({
      title: "Ese certificado no sirve para firmar",
    });
  });

  it("tells a locked PIN that it cannot sign until unlocked, and where a DNIe is unlocked", () => {
    expect(errorText("pinLocked", t)).toEqual({
      title: "La tarjeta tiene el PIN bloqueado",
      body: "No puedes firmar con ella hasta desbloquearlo. Si es un DNIe, se desbloquea en un punto de actualización del DNIe.",
    });
  });

  it.each(LANGUAGES)(
    "points a locked DNIe to its update points, never to a PUK, in %s",
    (language) => {
      const { body } = errorText("pinLocked", createI18n(language).t);

      expect(body).toContain("DNIe");
      expect(body).not.toMatch(/PUK/);
    },
  );

  it.each(SITUATIONS)("gives %s a translated title", (situation) => {
    const { title, body } = errorText(situation, t);

    expect(title).not.toMatch(/^errors\./);
    expect(body ?? "").not.toMatch(/^errors\./);
  });
});
