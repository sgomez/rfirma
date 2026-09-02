import { mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { po as poParser } from "gettext-parser";
import { describe, expect, it } from "vitest";
// @ts-expect-error `tools/` es JavaScript de construcción y queda fuera de tsconfig.
import { entriesOf, generate, isComplete, readCatalogs } from "../../tools/po-import.mjs";

/**
 * **Grada A**. El arnés del circuito de cadenas: TD-34 —un idioma a medias no
 * llega a existir— y las dos condiciones medidas del ID-127.
 */

const HEADER = [
  'msgid ""',
  'msgstr ""',
  '"Content-Type: text/plain; charset=UTF-8\\n"',
  '"Plural-Forms: nplurals=3; plural=(n==1 ? 0 : (n!=0 && n%1000000==0) ? 1 : 2);\\n"',
  "",
].join("\n");

/** Un `po/` de juguete: castellano completo y `eu` a medias a propósito. */
function fixture(basque: string): string {
  const directory = mkdtempSync(join(tmpdir(), "rfirma-po-"));
  writeFileSync(
    join(directory, "es.po"),
    `${HEADER}
msgid "actions.sign"
msgstr "Firmar documento"

msgid "panel.document.pages"
msgid_plural "panel.document.pages"
msgstr[0] "1 página"
msgstr[1] "{{count}} páginas"
msgstr[2] "{{count}} páginas"
`,
  );
  writeFileSync(join(directory, "eu.po"), `${HEADER}${basque}`);
  return directory;
}

const COMPLETE_BASQUE = `
msgid "actions.sign"
msgstr "Sinatu dokumentua"

msgid "panel.document.pages"
msgid_plural "panel.document.pages"
msgstr[0] "orrialde 1"
msgstr[1] "{{count}} orrialde"
msgstr[2] "{{count}} orrialde"
`;

describe("el importador de .po (TD-34)", () => {
  it("no genera el .ts del idioma que no está al 100 %", () => {
    const directory = fixture(`
msgid "actions.sign"
msgstr ""

msgid "panel.document.pages"
msgid_plural "panel.document.pages"
msgstr[0] "orrialde 1"
msgstr[1] "{{count}} orrialde"
msgstr[2] "{{count}} orrialde"
`);
    const output = mkdtempSync(join(tmpdir(), "rfirma-ts-"));

    expect(generate({ poDirectory: directory, outputDirectory: output })).toEqual(["es"]);
    expect(readdirSync(output).sort()).toEqual(["es.ts", "index.ts"]);
    // Y por eso no puede llegar al desplegable: no está en la lista.
    expect(readFileSync(join(output, "index.ts"), "utf8")).not.toContain('"eu"');
  });

  it("cuenta un `#, fuzzy` como no traducido, así que baja del 100 % (ID-126)", () => {
    const directory = fixture(
      COMPLETE_BASQUE.replace('msgid "actions.sign"', '#, fuzzy\nmsgid "actions.sign"'),
    );

    const basque = readCatalogs(directory).find((c: { tag: string }) => c.tag === "eu");
    expect(basque.complete).toBe(false);
    expect(
      generate({ poDirectory: directory, outputDirectory: mkdtempSync(join(tmpdir(), "x-")) }),
    ).toEqual(["es"]);
  });

  it("sí lo genera cuando llega al 100 %", () => {
    const output = mkdtempSync(join(tmpdir(), "rfirma-ts-"));

    expect(generate({ poDirectory: fixture(COMPLETE_BASQUE), outputDirectory: output })).toEqual([
      "es",
      "eu",
    ]);
  });

  it("`--all` rellena los huecos con el castellano, para quien traduce", () => {
    const directory = fixture(`
msgid "actions.sign"
msgstr ""

msgid "panel.document.pages"
msgid_plural "panel.document.pages"
msgstr[0] "orrialde 1"
msgstr[1] "{{count}} orrialde"
msgstr[2] "{{count}} orrialde"
`);
    const output = mkdtempSync(join(tmpdir(), "rfirma-ts-"));

    expect(generate({ poDirectory: directory, outputDirectory: output, all: true })).toEqual([
      "es",
      "eu",
    ]);
    expect(readFileSync(join(output, "eu.ts"), "utf8")).toContain('sign: "Firmar documento"');
  });

  it("desdobla cada plural en `_one`, `_many` y `_other`, en ese orden (ID-129)", () => {
    const po = poParser.parse(readFileSync(join(fixture(COMPLETE_BASQUE), "es.po")));

    expect(entriesOf(po)).toEqual([
      ["actions.sign", "Firmar documento"],
      ["panel.document.pages_one", "1 página"],
      ["panel.document.pages_many", "{{count}} páginas"],
      ["panel.document.pages_other", "{{count}} páginas"],
    ]);
  });

  it("emite `export default`, sin lo cual i18next-cli ve el idioma al 0 % (ID-127)", () => {
    const output = mkdtempSync(join(tmpdir(), "rfirma-ts-"));
    generate({ poDirectory: fixture(COMPLETE_BASQUE), outputDirectory: output });

    for (const name of ["es.ts", "eu.ts"]) {
      expect(readFileSync(join(output, name), "utf8"), name).toContain("export default");
    }
  });

  it("da por incompleta una entrada con solo espacios", () => {
    expect(isComplete([["a", "x"]])).toBe(true);
    expect(isComplete([["a", "   "]])).toBe(false);
  });
});
