import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

/**
 * **Grada A** (`vitest`, carril rápido). Sub-issue #177.
 *
 * `pdfjsLoader` no se puede montar en `jsdom` —importa `pdfjs-dist`, que quiere
 * un contexto `2d` y un worker—, así que lo que se comprueba aquí es **su
 * fuente**, leída del disco. Son guardias, no pruebas de comportamiento: vigilan
 * dos decisiones que se sostienen solas hasta el día en que alguien escriba una
 * línea de más, y ese día no daría ningún error.
 */

/**
 * La raíz del frontal, `src/`.
 *
 * Sale del directorio de trabajo y no de `import.meta.url` porque `vitest`
 * transforma el módulo y su URL deja de ser un `file:`; el directorio de
 * trabajo es `rfirma-app/`, que es desde donde se lanza la suite.
 */
const SOURCE_ROOT = join(process.cwd(), "src");

/**
 * Este mismo fichero no se recorre: está escrito con los literales que busca, y
 * leerse a sí mismo sería encontrarlos siempre. Es la misma exclusión que hace
 * `commands/guards.rs` con `THIS_FILE`.
 */
const THIS_FILE = "pdfjsLoader.test.ts";

/** Todo `.ts`/`.tsx` de producción bajo `src/`: sin pruebas y sin este fichero. */
function productionSources(directory: string = SOURCE_ROOT): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) return productionSources(path);
    if (!/\.tsx?$/.test(entry.name)) return [];
    if (/\.test\.tsx?$/.test(entry.name)) return [];
    if (entry.name === THIS_FILE) return [];
    return [path];
  });
}

describe("la vista previa depende de dos opciones de pdf.js", () => {
  /**
   * ID-138, TD-31. El sello que se ve dentro del recuadro lo pinta `pdf.js`
   * porque `page.render` deja `annotationMode` en su valor por omisión: la
   * apariencia del widget de firma es una anotación. Está medido que sólo
   * `DISABLE` (0) la apaga —`ENABLE_FORMS` no—, y que apagarla **no da ningún
   * error**: el recuadro se quedaría en blanco y nadie sabría por qué
   * (`docs/research/prefirma-en-seco-pdfjs.md`).
   *
   * Por eso la guardia no distingue valores: nadie escribe `annotationMode`, ni
   * siquiera el que hoy funciona. El día que el visor rellene formularios habrá
   * que escribirlo, y entonces esta prueba es dónde se discute.
   */
  it("no one sets annotationMode anywhere in the front end", () => {
    const writing = productionSources().filter((path) =>
      readFileSync(path, "utf8").includes("annotationMode"),
    );

    expect(writing).toEqual([]);
  });

  /**
   * ID-112. La apariencia del sello usa Courier, una de las catorce fuentes
   * estándar. Sin `standardFontDataUrl`, `pdf.js` avisa y sustituye: pinta
   * igual, pero con otras métricas, así que el corte de línea de la vista
   * previa no sería el del compositor. Las `standard_fonts` se empaquetan en
   * `vite.config.ts`; aquí se vigila que alguien se las pase.
   */
  it("hands pdf.js the standard fonts it packages", () => {
    const loader = readFileSync(join(SOURCE_ROOT, "viewer", "pdfjsLoader.ts"), "utf8");

    expect(loader).toContain("standardFontDataUrl");
  });
});
