import { describe, expect, it } from "vitest";
import { createI18n } from "../i18n/i18n";
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

  it.each(SITUATIONS)("gives %s a translated title", (situation) => {
    const { title, body } = errorText(situation, t);

    expect(title).not.toMatch(/^errors\./);
    expect(body ?? "").not.toMatch(/^errors\./);
  });
});
