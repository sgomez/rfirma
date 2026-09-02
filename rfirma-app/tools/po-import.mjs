/**
 * `po-import`: de los `.po` a los catálogos de TypeScript.
 *
 * Es el último tramo del circuito de cadenas del ADR-0009 (enmendado):
 *
 *     messages.pot ──msgmerge──▶ po/{es,ca,eu,gl,en}.po ──po-import──▶ src/i18n/locales/*.ts
 *        versionado                   versionados                    generados, NO versionados
 *
 * Tres reglas que no son detalles de implementación (ID-121…ID-130):
 *
 * - **Node puro.** La única dependencia es `gettext-parser`, así que un clon
 *   limpio compila con `pnpm install` y sin `gettext` instalado. `msgmerge` y
 *   `msgfmt` hacen falta para *desarrollar* y en el CI, no para construir.
 * - **El idioma que no está al 100 % no genera `.ts`.** No es una comprobación
 *   sino la forma del programa: `LANGUAGES` y `CATALOGS` salen de qué ficheros
 *   se han escrito. Un `#, fuzzy` cuenta como no traducido (ID-126).
 * - **`export default`.** El cargador de `i18next-cli` no ve una exportación
 *   con nombre y reportaría el idioma al 0 % con el fichero lleno delante
 *   (ID-127).
 *
 * `--all` genera además los idiomas incompletos, rellenando los huecos con el
 * castellano. Es para quien traduce y quiere ver su trabajo antes de llegar al
 * 100 %: **nunca en el CI ni en la publicación**.
 *
 * Uso: `node tools/po-import.mjs [--all]` desde `rfirma-app/`.
 */

import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { fileURLToPath } from "node:url";
import gettextParser from "gettext-parser";

/** El idioma de referencia: es la fuente de las claves y siempre se genera. */
export const REFERENCE_LANGUAGE = "es";

/**
 * Los sufijos de plural, en el orden en que van los `msgstr[n]` de cada
 * entrada plural.
 *
 * Son los del castellano —la lengua de referencia— y **se escriben en los
 * cinco idiomas por igual**, aunque el inglés no llegue a usar `_many`
 * (ID-129). La alternativa, un juego de sufijos por idioma, rompería
 * `Catalog = typeof es`, que es quien comprueba que las claves cuadran.
 *
 * Es también la razón de que la cabecera `Plural-Forms` sea decoración
 * (ID-122): quien reparte en tiempo de ejecución es `Intl.PluralRules` dentro
 * de i18next, no la expresión en C del `.po`.
 */
export const PLURAL_SUFFIXES = ["one", "many", "other"];

/** Los cinco idiomas, en el orden en que se enseñan (ID-124). */
export const LANGUAGE_ORDER = ["es", "ca", "eu", "gl", "en"];

/**
 * Las entradas de un `.po`, como pares `clave → texto` ya con el sufijo de
 * plural pegado a la clave.
 *
 * Un `msgstr` vacío no se omite: se devuelve como `""`, porque el catálogo
 * necesita la clave presente y `returnEmptyString: false` es quien la hace
 * caer al castellano (ID-130). Lo que sí se vacía es la traducción marcada
 * `#, fuzzy`: cuenta como no traducida (ID-126).
 */
export function entriesOf(po) {
  const entries = [];
  for (const context of Object.values(po.translations)) {
    for (const [msgid, translation] of Object.entries(context)) {
      if (msgid === "") continue; // la cabecera
      const fuzzy = (translation.comments?.flag ?? "").includes("fuzzy");
      if (translation.msgid_plural === undefined) {
        entries.push([msgid, fuzzy ? "" : (translation.msgstr[0] ?? "")]);
        continue;
      }
      PLURAL_SUFFIXES.forEach((suffix, index) => {
        entries.push([`${msgid}_${suffix}`, fuzzy ? "" : (translation.msgstr[index] ?? "")]);
      });
    }
  }
  return entries;
}

/** Un catálogo está completo cuando ninguna de sus cadenas está vacía. */
export function isComplete(entries) {
  return entries.every(([, text]) => text.trim() !== "");
}

/**
 * El catálogo anidado que espera i18next, a partir de las claves con puntos.
 *
 * El orden de las claves es el del `.po`, que a su vez es el de la plantilla:
 * así los cinco catálogos salen con las mismas claves en el mismo orden, que
 * es lo que compara `i18n.test.tsx`.
 */
export function nest(entries) {
  const root = {};
  for (const [key, text] of entries) {
    const path = key.split(".");
    const leaf = path.pop();
    let node = root;
    for (const step of path) {
      if (typeof node[step] !== "object" || node[step] === null) node[step] = {};
      node = node[step];
    }
    node[leaf] = text;
  }
  return root;
}

/** El literal de objeto, indentado como lo dejaría Biome. */
function renderObject(value, depth) {
  if (typeof value === "string") return JSON.stringify(value);
  const pad = "  ".repeat(depth + 1);
  const lines = Object.entries(value).map(
    ([key, child]) => `${pad}${renderKey(key)}: ${renderObject(child, depth + 1)},`,
  );
  return `{\n${lines.join("\n")}\n${"  ".repeat(depth)}}`;
}

/** Una clave se entrecomilla solo si no es un identificador. */
function renderKey(key) {
  return /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(key) ? key : JSON.stringify(key);
}

/**
 * El módulo de un catálogo.
 *
 * El castellano va **sin anotar**: de él sale el tipo `Catalog`, así que
 * anotarlo sería morderse la cola. Los demás se declaran `Catalog`, y es `tsc`
 * quien comprueba que no falta ni sobra una clave.
 */
export function renderCatalog(tag, catalog) {
  const banner = `// Generado por tools/po-import.mjs desde po/${tag}.po. No editar a mano:\n// las cadenas se traducen en el .po (ADR-0009 enmendado, ID-121).\n`;
  const body = renderObject(catalog, 0);
  if (tag === REFERENCE_LANGUAGE) {
    return `${banner}\n// El castellano es además la FORMA del catálogo: \`Catalog\` sale de aquí, así\n// que este es el único que no se anota.\nexport default ${body};\n`;
  }
  return `${banner}\nimport type { Catalog } from "../catalog";\n\nconst ${tag}: Catalog = ${body};\n\nexport default ${tag};\n`;
}

/**
 * El catálogo **en el formato exacto que escribe `i18next-cli`**.
 *
 * Es la instantánea contra la que corre `i18next-cli extract --ci`, y vive en
 * `node_modules/.cache/`, nunca en `src/`: la herramienta es **vigilante, no
 * dueña del catálogo** (ID-127), y `extract` reescribe el fichero que mira. Si
 * mirase los catálogos de verdad, se llevaría por delante el `: Catalog` que es
 * quien comprueba que las claves cuadran, y su `as const` haría literales las
 * hojas, con lo que `Catalog = typeof es` dejaría de admitir traducción alguna.
 *
 * El formato se replica al byte porque `--ci` sale con 1 ante *cualquier*
 * diferencia, también la de orden: claves ordenadas por su raíz —el sufijo de
 * plural no cuenta— y las formas de un plural seguidas, en el orden de
 * `Intl.PluralRules`. Lo fija `poImport.test.ts`.
 */
export function renderSnapshot(catalog) {
  return `export default ${JSON.stringify(sortLikeI18nextCli(catalog), null, 2)} as const;\n`;
}

/** Las categorías de plural, en el orden en que las ordena i18next. */
const PLURAL_ORDER = ["zero", "one", "two", "few", "many", "other"];

function sortLikeI18nextCli(value) {
  if (typeof value === "string") return value;
  const ranked = Object.entries(value).map(([key, child]) => {
    const suffix = PLURAL_ORDER.find((form) => key.endsWith(`_${form}`));
    const root = suffix === undefined ? key : key.slice(0, -(suffix.length + 1));
    return { key, child, root, form: suffix === undefined ? -1 : PLURAL_ORDER.indexOf(suffix) };
  });
  ranked.sort((a, b) => a.root.localeCompare(b.root) || a.form - b.form);
  return Object.fromEntries(ranked.map(({ key, child }) => [key, sortLikeI18nextCli(child)]));
}

/**
 * El índice de los catálogos generados.
 *
 * `LANGUAGES` y `CATALOGS` **se derivan de qué ficheros existen** (ID-123): un
 * idioma a medias no llega hasta aquí, así que «no publicar un idioma
 * incompleto» deja de ser una regla que alguien tiene que comprobar y pasa a
 * ser irrepresentable.
 */
export function renderIndex(tags) {
  const imports = tags.map((tag) => `import ${tag} from "./${tag}";`).join("\n");
  const list = tags.map((tag) => `"${tag}"`).join(", ");
  return `// Generado por tools/po-import.mjs. No editar a mano.\n//\n// Los idiomas que se publican son los que llegaron al 100 % en su .po: los\n// demás no tienen fichero, y por eso no pueden estar en esta lista (ID-123).\n\nimport type { Catalog } from "../catalog";\n${imports}\n\n/** Los idiomas publicados, en el orden en que se enseñan. */\nexport const LANGUAGES = [${list}] as const;\n\n/** El catálogo de cada idioma publicado. */\nexport const CATALOGS: Record<(typeof LANGUAGES)[number], Catalog> = { ${tags.join(", ")} };\n`;
}

/** Lee `po/`, devuelve `{ tag, entries, complete }` en el orden del ID-124. */
export function readCatalogs(poDirectory) {
  const found = readdirSync(poDirectory)
    .filter((name) => name.endsWith(".po"))
    .map((name) => basename(name, ".po"));
  const unknown = found.filter((tag) => !LANGUAGE_ORDER.includes(tag));
  if (unknown.length > 0) {
    throw new Error(`po/: idiomas fuera del ID-124: ${unknown.join(", ")}`);
  }
  return LANGUAGE_ORDER.filter((tag) => found.includes(tag)).map((tag) => {
    const entries = entriesOf(gettextParser.po.parse(readFileSync(join(poDirectory, `${tag}.po`))));
    return { tag, entries, complete: isComplete(entries) };
  });
}

/**
 * Genera los `.ts` y devuelve los idiomas publicados.
 *
 * Con `all`, los incompletos se generan también, rellenando cada hueco con el
 * castellano. Es lo único que hace `--all`, y por eso no puede colarse en la
 * publicación: el CI no lo pasa.
 */
export function generate({ poDirectory, outputDirectory, snapshotDirectory, all = false }) {
  const catalogs = readCatalogs(poDirectory);
  const reference = catalogs.find(({ tag }) => tag === REFERENCE_LANGUAGE);
  if (reference === undefined) throw new Error(`falta po/${REFERENCE_LANGUAGE}.po`);
  if (!reference.complete) {
    throw new Error(`po/${REFERENCE_LANGUAGE}.po no está completo, y es el original`);
  }
  const spanish = new Map(reference.entries);

  mkdirSync(outputDirectory, { recursive: true });
  if (snapshotDirectory !== undefined) mkdirSync(snapshotDirectory, { recursive: true });

  const published = [];
  for (const { tag, entries, complete } of catalogs) {
    if (!complete && !all) continue;
    const filled = entries.map(([key, text]) => [
      key,
      text === "" ? (spanish.get(key) ?? "") : text,
    ]);
    const catalog = nest(all ? filled : entries);
    writeFileSync(join(outputDirectory, `${tag}.ts`), renderCatalog(tag, catalog));
    if (snapshotDirectory !== undefined) {
      writeFileSync(join(snapshotDirectory, `${tag}.ts`), renderSnapshot(catalog));
    }
    published.push(tag);
  }
  writeFileSync(join(outputDirectory, "index.ts"), renderIndex(published));
  return published;
}

// Ejecutado como orden, no importado: `vitest` importa este módulo con una URL
// que no es `file:`, y `fileURLToPath` sobre ella revienta.
if (import.meta.url.startsWith("file:") && process.argv[1] === fileURLToPath(import.meta.url)) {
  const here = fileURLToPath(new URL(".", import.meta.url));
  const published = generate({
    poDirectory: join(here, "..", "po"),
    outputDirectory: join(here, "..", "src", "i18n", "locales"),
    snapshotDirectory: join(here, "..", "node_modules", ".cache", "i18next-cli"),
    all: process.argv.includes("--all"),
  });
  console.log(`catálogos generados: ${published.join(", ")}`);
}
