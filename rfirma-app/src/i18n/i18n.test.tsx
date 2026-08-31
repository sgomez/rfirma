import { act, render, screen } from "@testing-library/react";
import { useTranslation } from "react-i18next";
import { describe, expect, it } from "vitest";
import packageJson from "../../package.json" with { type: "json" };
import { catalogKeys, isComplete } from "./catalog";
import { createI18n } from "./i18n";
import { LanguageProvider, useLanguage } from "./LanguageProvider";
import { CATALOGS, completeLanguages, LANGUAGES } from "./languages";
import { es } from "./locales/es";
import { inMemoryLanguagePreference } from "./preference";

/** **Grada A** (`vitest`, carril rápido). ADR-0009 e ID-02, ID-28. */

describe("los seis catálogos", () => {
  it("cubre las seis lenguas del cliente oficial, y no un subconjunto", () => {
    expect([...LANGUAGES]).toEqual(["es", "ca", "eu", "gl", "va", "en"]);
  });

  it("tiene las mismas claves en los seis, también en los que no traducen", () => {
    const spanish = catalogKeys(es);

    for (const tag of LANGUAGES) {
      expect(catalogKeys(CATALOGS[tag]), `las claves de ${tag}`).toEqual(spanish);
    }
  });

  it("lleva contenido solo en castellano y en inglés", () => {
    expect(completeLanguages()).toEqual(["es", "en"]);
  });

  it("deja los cuatro cooficiales con la clave presente y sin traducir", () => {
    for (const tag of ["ca", "eu", "gl", "va"] as const) {
      expect(isComplete(CATALOGS[tag]), `${tag} no debería tener contenido en v0.1`).toBe(false);
    }
  });
});

describe("la resolución de cadenas", () => {
  it("cae al castellano donde el catálogo está sin traducir", () => {
    const i18n = createI18n("eu");

    // La clave existe en `eu` y está vacía: sin `returnEmptyString: false` esto
    // devolvería "" y la interfaz saldría en blanco en lugar de en español.
    expect(i18n.t("actions.sign")).toBe(es.actions.sign);
  });

  it("usa el idioma pedido cuando sí está traducido", () => {
    expect(createI18n("en").t("actions.sign")).toBe("Sign document");
    expect(createI18n("es").t("actions.sign")).toBe("Firmar documento");
  });

  it("no olfatea el idioma: no hay detector de idioma en las dependencias", () => {
    const dependencies = {
      ...packageJson.dependencies,
      ...packageJson.devDependencies,
    } as Record<string, string>;

    expect(Object.keys(dependencies)).not.toContain("i18next-browser-languagedetector");
  });

  it("no trae Tailwind, que sería un segundo sistema de color", () => {
    const dependencies = {
      ...packageJson.dependencies,
      ...packageJson.devDependencies,
    } as Record<string, string>;

    expect(Object.keys(dependencies).filter((name) => name.includes("tailwind"))).toEqual([]);
  });
});

function Probe() {
  const { t } = useTranslation();
  const { language, setLanguage } = useLanguage();

  return (
    <div>
      <p>{t("actions.sign")}</p>
      <p>{language}</p>
      <button type="button" onClick={() => void setLanguage("en")}>
        {t("actions.change")}
      </button>
    </div>
  );
}

describe("el cambio de idioma", () => {
  it("cambia la interfaz sin reiniciar y guarda la preferencia", async () => {
    const preference = inMemoryLanguagePreference("es");
    const i18n = createI18n(await preference.read());
    render(
      <LanguageProvider i18n={i18n} preference={preference}>
        <Probe />
      </LanguageProvider>,
    );

    expect(screen.getByText("Firmar documento")).toBeInTheDocument();

    await act(async () => {
      screen.getByRole("button").click();
    });

    expect(screen.getByText("Sign document")).toBeInTheDocument();
    expect(screen.getByText("en")).toBeInTheDocument();
    await expect(preference.read()).resolves.toBe("en");
  });

  it("arranca en el idioma que dice la preferencia guardada", async () => {
    const preference = inMemoryLanguagePreference("en");
    const i18n = createI18n(await preference.read());
    render(
      <LanguageProvider i18n={i18n} preference={preference}>
        <Probe />
      </LanguageProvider>,
    );

    expect(screen.getByText("Sign document")).toBeInTheDocument();
  });
});
