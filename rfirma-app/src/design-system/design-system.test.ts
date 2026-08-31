import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

/**
 * **Grada A** (`vitest`, carril rápido).
 *
 * `docs/design/design-system.md` es normativa: una desviación respecto de la
 * ficha es un error, no una mejora (#55). Estas pruebas leen la ficha y el CSS
 * y comparan, de modo que quien cambie uno sin el otro lo sabe al momento.
 * Nadie tiene que acordarse de actualizar una lista escrita a mano aquí.
 */

const read = (relative: string) =>
  readFileSync(fileURLToPath(new URL(relative, import.meta.url)), "utf8");

/** Los comentarios se quitan antes de mirar nada: llevan ejemplos de
 * selectores y de colores, y un `indexOf` los confundiría con el CSS real. */
const stripComments = (css: string) => css.replace(/\/\*[\s\S]*?\*\//g, "");

const specification = read("../../../docs/design/design-system.md");
const tokens = stripComments(read("./tokens.css"));
const components = stripComments(read("./components.css"));

/** Sin espacios y con el cero decimal fuera: `rgba(0, 0, 0, 0.6)` y
 * `rgba(0,0,0,.6)` son el mismo color, y el formateador de Biome elige uno. */
const normalizeColor = (value: string) => value.replace(/\s+/g, "").replace(/0\.(\d)/g, ".$1");

/**
 * Los roles de color de la sección 2, con su valor en cada tema. `igual` en la
 * columna clara significa que el rol no cambia con el tema.
 */
function colorRolesFromSpecification(): Array<{ role: string; dark: string; light: string }> {
  const roles = [];
  for (const line of specification.split("\n")) {
    const row = /^\|\s*`(--rf-[a-z-]+)`\s*\|([^|]*)\|([^|]*)\|/.exec(line);
    if (!row) continue;
    const [, role, darkCell = "", lightCell = ""] = row;
    const dark = /`([^`]+)`/.exec(darkCell)?.[1];
    const light = /`([^`]+)`/.exec(lightCell)?.[1] ?? (/igual/.test(lightCell) ? dark : undefined);
    if (!role || !dark || !light) continue;
    roles.push({ role, dark: normalizeColor(dark), light: normalizeColor(light) });
  }
  return roles;
}

/**
 * Las declaraciones de la primera regla cuyo selector contiene `anchor`. Vale
 * porque en `tokens.css` ninguna regla anida otra: el `@media` envuelve una
 * sola regla y el corte por la primera llave de cierre cae donde debe.
 */
function declarationsOf(css: string, anchor: string): Map<string, string> {
  const start = css.indexOf(anchor);
  expect(start, `no hay ninguna regla con el selector ${anchor}`).toBeGreaterThan(-1);
  const open = css.indexOf("{", start);
  const block = css.slice(open + 1, css.indexOf("}", open));
  const declarations = new Map<string, string>();
  for (const declaration of block.split(";")) {
    const parsed = /^\s*(--rf-[a-z0-9-]+)\s*:\s*([^;]+)$/.exec(declaration);
    if (parsed?.[1] && parsed[2]) declarations.set(parsed[1], normalizeColor(parsed[2]));
  }
  return declarations;
}

describe("los tokens de color", () => {
  const roles = colorRolesFromSpecification();

  it("cubre la tabla entera de la ficha, y no una parte", () => {
    // Doce roles en la sección 2. La cifra está aquí para que un fallo de
    // parseo se vea como tal en lugar de dar por buena una tabla vacía.
    expect(roles.map(({ role }) => role)).toHaveLength(12);
  });

  it("define todos los roles en el bloque claro, que es el bloque base", () => {
    const light = declarationsOf(tokens, '[data-theme="light"]');

    for (const { role, light: value } of roles) {
      expect(light.get(role), `${role} en el tema claro`).toBe(value);
    }
  });

  it("redefine todos los roles cuando el sistema pide oscuro", () => {
    const dark = declarationsOf(tokens, ':root:not([data-theme="light"])');

    for (const { role, dark: value } of roles) {
      expect(dark.get(role), `${role} en el tema oscuro automático`).toBe(value);
    }
  });

  it("redefine todos los roles cuando el tema oscuro se fuerza", () => {
    const dark = declarationsOf(tokens, '[data-theme="dark"]');

    for (const { role, dark: value } of roles) {
      expect(dark.get(role), `${role} en el tema oscuro forzado`).toBe(value);
    }
  });

  it("no deja ningún rol viviendo solo dentro de la media query", () => {
    // Invariante 1 de la sección 1: el bloque claro es el bloque base.
    const light = declarationsOf(tokens, '[data-theme="light"]');
    const dark = declarationsOf(tokens, ':root:not([data-theme="light"])');

    for (const role of dark.keys()) {
      expect(light.has(role), `${role} solo existe en oscuro`).toBe(true);
    }
  });

  it("repinta el fondo con especificidad 0, para no aplastar una superficie", () => {
    // Invariante 3 de la sección 1.
    expect(tokens).toContain(":where([data-theme])");
  });
});

describe("los tokens sin tema", () => {
  it("tiene los nueve escalones de espaciado de la ficha", () => {
    const scale = ["8px", "16px", "24px", "40px", "48px", "64px", "72px", "80px", "144px"];
    const declared = declarationsOf(tokens, ":root {");

    scale.forEach((value, index) => {
      expect(declared.get(`--rf-space-${index + 1}`)).toBe(value);
    });
  });

  it("resuelve los alias semánticos sobre la escala, sin px sueltos", () => {
    const declared = declarationsOf(tokens, ":root {");
    const aliases = {
      "--rf-space-xs": "--rf-space-1",
      "--rf-space-sm": "--rf-space-2",
      "--rf-space-md": "--rf-space-3",
      "--rf-space-lg": "--rf-space-5",
      "--rf-space-xl": "--rf-space-8",
      "--rf-space-2xl": "--rf-space-9",
    };

    for (const [alias, step] of Object.entries(aliases)) {
      expect(declared.get(alias)).toBe(`var(${step})`);
    }
  });

  it("declara los radios, las dos elevaciones y los puntos de ruptura", () => {
    const declared = declarationsOf(tokens, ":root {");
    const expected = [
      "--rf-radius-sm",
      "--rf-radius-md",
      "--rf-radius-lg",
      "--rf-radius-xl",
      "--rf-radius-pill",
      "--rf-shadow-card",
      "--rf-shadow-elevated",
      "--rf-bp-xs",
      "--rf-bp-sm",
      "--rf-bp-md",
      "--rf-bp-lg",
      "--rf-bp-xl",
      "--rf-bp-2xl",
      "--rf-font-display",
      "--rf-font-body",
    ];

    for (const token of expected) {
      expect(declared.has(token), `falta ${token}`).toBe(true);
    }
  });

  it("tiene dos escalones de movimiento reales, no tres", () => {
    // Sección 6 y nota final de la sección 11.
    const declared = declarationsOf(tokens, ":root {");

    expect(declared.get("--rf-duration-fast")).toBe("150ms");
    expect(declared.get("--rf-duration-base")).toBe("300ms");
    expect(declared.get("--rf-duration-slow")).toBe("var(--rf-duration-base)");
  });

  it("apaga el movimiento desde la capa de tokens y no desde los componentes", () => {
    expect(tokens).toContain("prefers-reduced-motion");
    expect(components).not.toContain("prefers-reduced-motion");
  });
});

describe("el vocabulario de clases", () => {
  /**
   * La sección 9 escribe las familias en forma comprimida
   * (`.rf-btn` + `--primary\|--secondary`), así que la tabla se lee a mano.
   * Fuera de esta lista no hay clases: la maquetación propia se escribe con
   * `var(--rf-*)`.
   */
  const vocabulary = new Set([
    "rf-root",
    "rf-display",
    "rf-heading",
    "rf-title",
    "rf-body",
    "rf-prose",
    "rf-text-muted",
    "rf-text-primary",
    "rf-stack",
    "rf-row",
    "rf-section",
    "rf-divider",
    "rf-gap-xs",
    "rf-gap-sm",
    "rf-gap-md",
    "rf-gap-lg",
    "rf-surface",
    "rf-card",
    "rf-card--elevated",
    "rf-card--interactive",
    "rf-btn",
    "rf-btn--primary",
    "rf-btn--secondary",
    "rf-btn--ghost",
    "rf-btn--pill",
    "rf-field",
    "rf-field--error",
    "rf-label",
    "rf-input",
    "rf-hint",
    "rf-badge",
    "rf-badge--primary",
    "rf-dialog",
    "rf-scrim",
  ]);

  const declared = new Set(
    [...components.matchAll(/\.(rf-[a-z0-9-]+)/g)].map(([, name]) => name as string),
  );

  it("está implementado entero", () => {
    for (const className of vocabulary) {
      expect(declared.has(className), `falta la clase .${className}`).toBe(true);
    }
  });

  it("está cerrado: no hay clases rf- fuera de la tabla", () => {
    for (const className of declared) {
      expect(vocabulary.has(className), `.${className} no está en la sección 9`).toBe(true);
    }
  });

  it("no fija ningún color a mano: todo sale de los roles", () => {
    // El error más fácil de cometer, según la sección 1.
    const literals = components.match(/#[0-9a-f]{3,8}\b|\brgba?\(/gi) ?? [];

    expect(literals).toEqual([]);
  });

  it("da a los controles el área táctil y el contorno que exige la sección 8", () => {
    expect(components).toContain("min-height: 44px");
    expect(components).toContain(":focus-visible");
    // El contorno de control es el borde fuerte, nunca el sutil (sección 2).
    const buttonSecondary = components.slice(components.indexOf(".rf-btn--secondary"));

    expect(buttonSecondary.slice(0, 400)).toContain("--rf-border-strong");
  });
});
