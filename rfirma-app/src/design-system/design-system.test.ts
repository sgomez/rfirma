import { existsSync, readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

/**
 * **Grada A** (`vitest`, carril rápido).
 *
 * Desde el #85 el CSS de la aplicación **es** el bundle del proyecto de sistema
 * de diseño, versionado en `bundle/` (ID-47). El bundle manda y
 * `docs/design/design-system.md` lo describe; estas pruebas leen los dos y
 * comparan, de modo que quien cambie uno sin el otro lo sabe al momento
 * (TD-12). Nadie tiene que acordarse de actualizar una lista escrita a mano
 * aquí.
 *
 * Lo que el sello (`check-bundle.sh`) no puede saber —que el bundle siga
 * diciendo lo que la ficha promete— lo sabe esto; lo que esto no puede saber
 * —que nadie haya editado el bundle en el sitio equivocado— lo sabe el sello.
 */

const read = (relative: string) =>
  readFileSync(fileURLToPath(new URL(relative, import.meta.url)), "utf8");

const exists = (relative: string) => existsSync(fileURLToPath(new URL(relative, import.meta.url)));

/** Los comentarios se quitan antes de mirar nada: llevan ejemplos de
 * selectores y de colores, y un `indexOf` los confundiría con el CSS real. */
const stripComments = (css: string) => css.replace(/\/\*[\s\S]*?\*\//g, "");

const specification = read("../../../docs/design/design-system.md");

/**
 * La hoja raíz del bundle y su cierre de imports, en ese orden. Se lee de
 * `styles.css` en vez de escribirse aquí: si el bundle deja de importar un
 * fichero de tokens, la prueba mira lo que la aplicación recibe de verdad y no
 * una lista nuestra que ya no es cierta.
 */
const bundleRoot = read("./bundle/styles.css");
const imported = [...bundleRoot.matchAll(/@import\s+url\(['"]\.\/([^'"]+)['"]\)/g)].map(
  ([, path]) => path as string,
);

const partOfBundle = (path: string) => stripComments(read(`./bundle/${path}`));

/** Los tokens: todo lo que importa `styles.css` menos la capa de componentes y
 * la de tipografía, concatenado en el orden en que la cascada los ve. */
const tokens = imported
  .filter((path) => path.startsWith("tokens/"))
  .map(partOfBundle)
  .join("\n");
const components = partOfBundle("_ds_bundle.css");
const fonts = partOfBundle("fonts/fonts.css");

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

/** Las declaraciones de custom properties de un bloque de reglas. */
function declarationsIn(block: string): Map<string, string> {
  const declarations = new Map<string, string>();
  for (const declaration of block.split(";")) {
    const parsed = /^\s*(--rf-[a-z0-9-]+)\s*:\s*([^;]+)$/.exec(declaration);
    if (parsed?.[1] && parsed[2]) declarations.set(parsed[1], normalizeColor(parsed[2]));
  }
  return declarations;
}

/**
 * Las declaraciones de la primera regla cuyo selector contiene `anchor`. Vale
 * porque en la capa de tokens ninguna regla anida otra: el `@media` envuelve
 * una sola regla y el corte por la primera llave de cierre cae donde debe.
 */
function declarationsOf(css: string, anchor: string): Map<string, string> {
  const start = css.indexOf(anchor);
  expect(start, `no hay ninguna regla con el selector ${anchor}`).toBeGreaterThan(-1);
  const open = css.indexOf("{", start);
  return declarationsIn(css.slice(open + 1, css.indexOf("}", open)));
}

/** Fuera las `@media` enteras. Cada una envuelve una sola regla. */
const withoutMediaQueries = (css: string) => css.replace(/@media[^{]*\{[\s\S]*?\}\s*\}/g, "");

/**
 * Todo lo que declaran los bloques `:root` sin tema, fundidos. El bundle los
 * reparte entre siete ficheros —espaciado, radio, sombra…— y aquí interesa el
 * conjunto, que es lo que ve la cascada.
 */
function themelessDeclarations(css: string): Map<string, string> {
  const merged = new Map<string, string>();
  const plain = withoutMediaQueries(css);
  // `:root\s*\{` no puede confundirse con `:root:not(...)` ni con el `:root,`
  // que abre el bloque claro: los dos llevan algo distinto de `{` detrás.
  for (const rule of plain.matchAll(/:root\s*\{([^}]*)\}/g)) {
    for (const [name, value] of declarationsIn(rule[1] ?? "")) merged.set(name, value);
  }
  expect(merged.size, "no se ha leído ningún bloque :root de la capa de tokens").toBeGreaterThan(0);
  return merged;
}

/** Un rol se resuelve en su bloque de tema o, si no cambia con el tema, en los
 * `:root` sin tema. `--rf-scrim` es el único así hoy. */
function rolesOf(anchor: string): Map<string, string> {
  const resolved = themelessDeclarations(tokens);
  for (const [name, value] of declarationsOf(tokens, anchor)) resolved.set(name, value);
  return resolved;
}

describe("los tokens de color", () => {
  const roles = colorRolesFromSpecification();

  it("cubre la tabla entera de la ficha, y no una parte", () => {
    // Doce roles en la sección 2. La cifra está aquí para que un fallo de
    // parseo se vea como tal en lugar de dar por buena una tabla vacía.
    expect(roles.map(({ role }) => role)).toHaveLength(12);
  });

  it("define todos los roles en el bloque claro, que es el bloque base", () => {
    const light = rolesOf('[data-theme="light"]');

    for (const { role, light: value } of roles) {
      expect(light.get(role), `${role} en el tema claro`).toBe(value);
    }
  });

  it("redefine todos los roles cuando el sistema pide oscuro", () => {
    const dark = rolesOf(':root:not([data-theme="light"])');

    for (const { role, dark: value } of roles) {
      expect(dark.get(role), `${role} en el tema oscuro automático`).toBe(value);
    }
  });

  it("redefine todos los roles cuando el tema oscuro se fuerza", () => {
    const dark = rolesOf('[data-theme="dark"]');

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

  it("declara `color-scheme` en los tres bloques de tema", () => {
    // Sin él, el navegador pinta con el esquema del sistema los controles
    // nativos —barras de desplazamiento, autocompletado— y una zona forzada al
    // tema contrario se ve con la barra del otro tema.
    for (const anchor of [
      '[data-theme="light"]',
      ':root:not([data-theme="light"])',
      '[data-theme="dark"]',
    ]) {
      const start = tokens.indexOf(anchor);
      const open = tokens.indexOf("{", start);
      const block = tokens.slice(open + 1, tokens.indexOf("}", open));

      expect(block, `color-scheme en ${anchor}`).toContain("color-scheme:");
    }
  });

  it("conserva los literales de la paleta, que no cambian con el tema", () => {
    // Son referencia, no consumo: lo que se usa son los roles. Estaban en el
    // bundle y la reescritura a mano los perdió (ID-47).
    const declared = themelessDeclarations(tokens);
    const literals = [
      "--rf-color-primary",
      "--rf-color-on-primary",
      "--rf-color-background",
      "--rf-color-surface",
      "--rf-color-border",
      "--rf-color-text",
      "--rf-color-text-muted",
      "--rf-color-accent",
    ];

    for (const token of literals) {
      expect(declared.has(token), `falta ${token}`).toBe(true);
    }
  });

  it("mantiene `.rf-on-light` como alias del tema claro", () => {
    // Sección 9. Es un alias de `data-theme="light"`, así que tiene que estar
    // en el MISMO selector: uno que se quedara atrás sería un tema a medias.
    const light = tokens.slice(tokens.indexOf('[data-theme="light"]'));

    expect(light.slice(0, light.indexOf("{"))).toContain(".rf-on-light");
  });

  it("repinta el fondo con especificidad 0, para no aplastar una superficie", () => {
    // Invariante 3 de la sección 1.
    expect(components).toContain(":where([data-theme]");
  });
});

describe("los tokens sin tema", () => {
  it("tiene los nueve escalones de espaciado de la ficha", () => {
    const scale = ["8px", "16px", "24px", "40px", "48px", "64px", "72px", "80px", "144px"];
    const declared = themelessDeclarations(tokens);

    scale.forEach((value, index) => {
      expect(declared.get(`--rf-space-${index + 1}`)).toBe(value);
    });
  });

  it("resuelve los alias semánticos sobre la escala, sin px sueltos", () => {
    const declared = themelessDeclarations(tokens);
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
    const declared = themelessDeclarations(tokens);
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

  it("da las dos sombras en cuatro capas", () => {
    // Lo que la reescritura a mano aplanó a un `0 1px 2px` (ID-47). Las capas
    // vacías del principio no son ruido: son las ranuras que el sistema deja
    // para un anillo y un contorno, y quitarlas cambia el orden de pintado.
    const declared = themelessDeclarations(tokens);

    for (const shadow of ["--rf-shadow-card", "--rf-shadow-elevated"]) {
      const layers = (declared.get(shadow) ?? "").split(/,(?![^(]*\))/);

      expect(layers, `capas de ${shadow}`).toHaveLength(4);
    }
  });

  it("da al anillo de foco su grosor y su desplazamiento como tokens", () => {
    // Sección 8. La capa de componentes los consume con `var()`, así que un
    // literal aquí sería un anillo que no se puede ajustar desde los tokens.
    const declared = themelessDeclarations(tokens);

    expect(declared.get("--rf-focus-ring-width")).toBe("2px");
    expect(declared.get("--rf-focus-ring-offset")).toBe("2px");
    expect(components).toContain("var(--rf-focus-ring-width)");
    expect(components).toContain("var(--rf-focus-ring-offset)");
  });

  it("tiene dos escalones de movimiento reales, no tres", () => {
    // Sección 6 y nota final de la sección 12.
    const declared = themelessDeclarations(tokens);

    expect(declared.get("--rf-duration-fast")).toBe("150ms");
    expect(declared.get("--rf-duration-base")).toBe("300ms");
    expect(declared.get("--rf-duration-slow")).toBe(declared.get("--rf-duration-base"));
  });

  it("apaga el movimiento desde la capa de tokens y no desde los componentes", () => {
    expect(tokens).toContain("prefers-reduced-motion");
    expect(components).not.toContain("prefers-reduced-motion");
  });
});

describe("la tipografía", () => {
  const faces = [...fonts.matchAll(/@font-face\s*\{([^}]*)\}/g)].map(
    ([, block]) => block as string,
  );

  it("declara al menos una `@font-face`", () => {
    expect(faces.length).toBeGreaterThan(0);
  });

  it("sirve cada `@font-face` desde un fichero local que existe en el árbol", () => {
    // ID-49: dentro del sandbox del flatpak no hay red, así que una `src` que
    // apunte fuera es una tipografía que nunca carga.
    for (const face of faces) {
      const source = /url\(\s*['"]?([^'")]+)['"]?\s*\)/.exec(face)?.[1];

      expect(source, `una @font-face sin src:\n${face}`).toBeDefined();
      expect(source?.startsWith("./"), `${source} no es una ruta relativa`).toBe(true);
      expect(exists(`./bundle/fonts/${source?.slice(2)}`), `no existe ${source}`).toBe(true);
    }
  });

  it("no pide nada por la red desde ninguna hoja del bundle", () => {
    // El `@import` a Google Fonts que traía el bundle es el caso concreto; la
    // prueba es general porque cualquier otro origen remoto falla igual.
    const everything = [bundleRoot, tokens, components, fonts].join("\n");

    expect(everything).not.toMatch(/https?:/);
    expect(everything).not.toMatch(/url\(\s*['"]?\/\//);
  });

  it("se acompaña de su licencia OFL", () => {
    expect(exists("./bundle/fonts/OFL.txt")).toBe(true);
    expect(read("./bundle/fonts/OFL.txt")).toContain("SIL Open Font License");
  });

  it("degrada a la sans del sistema y nunca a serif", () => {
    // ID-50. Un fallo de carga que cayera a serif cambiaría la aplicación
    // entera de voz, y es lo que hace el navegador si la pila no dice otra cosa.
    for (const role of ["--rf-font-display", "--rf-font-body"]) {
      const stack = new RegExp(`${role}\\s*:\\s*([^;]+);`).exec(tokens)?.[1] ?? "";

      expect(stack, `pila de ${role}`).toContain("Inter");
      expect(stack, `${role} no declara respaldos del sistema`).toContain("system-ui");
      expect(stack.trim().endsWith("sans-serif"), `${role} acaba en "${stack.trim()}"`).toBe(true);
      expect(/\bserif\b(?!-)/.test(stack.replace(/sans-serif/g, "")), `${role} cae a serif`).toBe(
        false,
      );
    }
  });
});

describe("las sombras del repositorio", () => {
  // ID-48: el bundle manda y el `<helmet>` de los artboards es una copia
  // comprimida, no la fuente. Un solo valor por token en todo el repositorio.
  const declared = themelessDeclarations(tokens);
  const artboardsDir = "../../../docs/design/artboards/";
  const artboards = readdirSync(fileURLToPath(new URL(artboardsDir, import.meta.url)))
    .map((name) => read(artboardsDir + name))
    .join("\n");

  it.each(["--rf-shadow-card", "--rf-shadow-elevated"])(
    "%s vale lo mismo en los artboards que en el bundle",
    (shadow) => {
      const copies = [...artboards.matchAll(new RegExp(`${shadow}\\s*:\\s*([^;]+);`, "g"))].map(
        ([, value]) => normalizeColor(value as string),
      );

      expect(copies.length, `ningún artboard declara ${shadow}`).toBeGreaterThan(0);
      for (const copy of copies) {
        expect(copy).toBe(declared.get(shadow));
      }
    },
  );
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
    "rf-on-light",
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
    "rf-btn--disabled",
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
