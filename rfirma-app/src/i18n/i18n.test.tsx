import { readFileSync } from "node:fs";
import { act, render, screen } from "@testing-library/react";
import { po as poParser } from "gettext-parser";
import { useTranslation } from "react-i18next";
import { describe, expect, it } from "vitest";
import packageJson from "../../package.json" with { type: "json" };
import { catalogKeys, catalogValues } from "./catalog";
import { createI18n } from "./i18n";
import { LanguageProvider, useLanguage } from "./LanguageProvider";
import { CATALOGS, LANGUAGES } from "./languages";
import es from "./locales/es";
import { inMemoryLanguagePreference } from "./preference";

/** **Grada A** (`vitest`, carril rápido). ADR-0009 e ID-02, ID-28, ID-121…ID-130. */

/** Los sufijos que i18next elige con `Intl.PluralRules` (ID-129). */
const PLURAL_SUFFIXES = ["_one", "_many", "_other"];

describe("el circuito de cadenas", () => {
  it("publica solo idiomas de la lista de cinco, y el castellano siempre", () => {
    expect(LANGUAGES).toContain("es");
    for (const tag of LANGUAGES) {
      expect(["es", "ca", "eu", "gl", "en"], `${tag} no es de los cinco (ID-124)`).toContain(tag);
    }
  });

  it("no publica el valencià, que salió con los plurales (ID-124)", () => {
    // La invariante nuestra es la lista de cinco. Lo que motivó la salida del
    // valencià —que sus categorías de plural no son las del castellano— se
    // afirma aquí en la única forma que no depende del ICU del intérprete:
    // `va` no tiene `many`, que es la categoría que `es` y `ca` sí usan. El
    // conjunto exacto que devuelve `Intl.PluralRules("va")` **sí** varía con la
    // versión de CLDR (`["other"]` con un ICU que no conoce `va`,
    // `["one","other"]` con uno que sí), y afirmarlo rompía el CI solo.
    expect([...LANGUAGES]).not.toContain("va");
    expect(new Intl.PluralRules("va").resolvedOptions().pluralCategories).not.toContain("many");
    expect(new Intl.PluralRules("es").resolvedOptions().pluralCategories).toContain("many");
  });

  it("tiene las mismas claves en todos los publicados", () => {
    const spanish = catalogKeys(es);

    for (const tag of LANGUAGES) {
      expect(catalogKeys(CATALOGS[tag]), `las claves de ${tag}`).toEqual(spanish);
    }
  });

  it("no publica un idioma con una sola cadena vacía (ID-123)", () => {
    for (const tag of LANGUAGES) {
      const empty = catalogKeys(CATALOGS[tag]).filter(
        (_, index) => (catalogValues(CATALOGS[tag])[index] ?? "").trim() === "",
      );
      expect(empty, `${tag} llegó al desplegable con huecos`).toEqual([]);
    }
  });

  it("reserva el `_` para el plural: ninguna otra clave lo lleva (ID-130)", () => {
    const offenders = catalogKeys(es)
      .map((key) => key.split(".").at(-1) ?? "")
      .filter((leaf) => leaf.includes("_"))
      .filter((leaf) => !PLURAL_SUFFIXES.some((suffix) => leaf.endsWith(suffix)));

    expect(offenders).toEqual([]);
  });

  it("escribe las tres formas del castellano en cada plural, `_many` incluido", () => {
    // `es` y `ca` tienen tres categorías, no dos, y olvidar `_many` es un error
    // silencioso: 1 000 000 cae en `many` y sin la clave sale la forma `other`.
    expect(new Intl.PluralRules("es").resolvedOptions().pluralCategories.sort()).toEqual([
      "many",
      "one",
      "other",
    ]);

    const leaves = catalogKeys(es).map((key) => key.split(".").at(-1) ?? "");
    const roots = new Set(
      leaves.filter((leaf) => leaf.endsWith("_one")).map((leaf) => leaf.slice(0, -"_one".length)),
    );
    expect(roots.size).toBeGreaterThan(0);
    for (const root of roots) {
      for (const suffix of PLURAL_SUFFIXES) {
        expect(leaves, `falta ${root}${suffix}`).toContain(`${root}${suffix}`);
      }
    }
  });
});

describe("la plantilla de gettext", () => {
  const template = poParser.parse(readFileSync("po/messages.pot"));
  const msgids = Object.keys(template.translations[""] ?? {}).filter((msgid) => msgid !== "");

  it("usa la clave con puntos como `msgid`, no el texto castellano (ID-122)", () => {
    expect(msgids.length).toBeGreaterThan(0);
    for (const msgid of msgids) {
      expect(msgid, `${msgid} no parece una clave`).toMatch(
        /^[a-zA-Z][a-zA-Z0-9]*(\.[a-zA-Z][a-zA-Z0-9]*)*$/,
      );
    }
  });

  it("no usa `msgctxt`: con `msgid` = clave el contexto es redundante (ID-122)", () => {
    expect(Object.keys(template.translations)).toEqual([""]);
  });

  it("deja los `msgstr` vacíos, o `msgmerge` daría el castellano por traducción", () => {
    for (const [msgid, translation] of Object.entries(template.translations[""] ?? {})) {
      if (msgid === "") continue;
      expect(translation.msgstr.join(""), `${msgid} trae texto`).toBe("");
    }
  });

  it("no lleva `_` en ninguna clave: el sufijo lo pone el importador (ID-130)", () => {
    expect(msgids.filter((msgid) => msgid.includes("_"))).toEqual([]);
  });
});

describe("la resolución de cadenas", () => {
  it("reparte los plurales con `Intl.PluralRules`, no con un ternario", () => {
    const i18n = createI18n("es");

    expect(i18n.t("panel.document.pages", { count: 1 })).toBe("1 página");
    expect(i18n.t("panel.document.pages", { count: 2 })).toBe("2 páginas");
    expect(i18n.t("panel.document.pages", { count: 1_000_000 })).toBe("1000000 páginas");
  });

  it("mantiene `pin.incorrectUnknown` fuera del plural: es otro mensaje (ID-129)", () => {
    const i18n = createI18n("es");

    expect(i18n.t("pin.incorrectUnknown")).not.toContain("{{count}}");
    expect(i18n.t("pin.incorrectUnknown")).not.toBe(i18n.t("pin.incorrect", { count: 3 }));
  });

  it("cae al castellano donde el catálogo está sin traducir", () => {
    const i18n = createI18n("es");

    // `returnEmptyString: false` es normativo (ID-130): sin él, una cadena
    // vacía se daría por buena y la interfaz saldría en blanco.
    expect(i18n.options.returnEmptyString).toBe(false);
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

  it("jubila a i18next-parser: quien mira el código es i18next-cli (ID-127)", () => {
    const dependencies = {
      ...packageJson.dependencies,
      ...packageJson.devDependencies,
    } as Record<string, string>;

    expect(Object.keys(dependencies)).not.toContain("i18next-parser");
    // Publica 24 veces en 30 días: la versión va fijada exacta, sin `^`.
    expect(dependencies["i18next-cli"]).toMatch(/^\d+\.\d+\.\d+$/);
    expect(dependencies["gettext-parser"]).toMatch(/^\d+\.\d+\.\d+$/);
  });

  it("no trae Tailwind, que sería un segundo sistema de color", () => {
    const dependencies = {
      ...packageJson.dependencies,
      ...packageJson.devDependencies,
    } as Record<string, string>;

    expect(Object.keys(dependencies).filter((name) => name.includes("tailwind"))).toEqual([]);
  });
});

function Probe({ onFailure }: { onFailure?: (thrown: unknown) => void }) {
  const { t } = useTranslation();
  const { language, setLanguage } = useLanguage();

  return (
    <div>
      <p>{t("actions.sign")}</p>
      <p>{language}</p>
      <button type="button" onClick={() => void setLanguage("en").catch(onFailure)}>
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

  it("puts the language back and rethrows when the disk refuses to save it", async () => {
    const preference = {
      read: async () => "es" as const,
      save: async () => {
        throw new Error("disco lleno");
      },
    };
    const i18n = createI18n(await preference.read());
    let failure: unknown = null;
    render(
      <LanguageProvider i18n={i18n} preference={preference}>
        <Probe onFailure={(thrown) => (failure = thrown)} />
      </LanguageProvider>,
    );

    await act(async () => {
      screen.getByRole("button").click();
    });

    // El rechazo llega a quien lo tiene que contar…
    expect(failure).toBeInstanceOf(Error);
    // …y la ventana ha vuelto al idioma anterior, que es lo que ese aviso dice.
    expect(screen.getByText("Firmar documento")).toBeInTheDocument();
    expect(screen.getByText("es")).toBeInTheDocument();
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
