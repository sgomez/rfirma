import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { es } from "./es";

const source = fileURLToPath(new URL("..", import.meta.url));

function astroFiles(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      return astroFiles(path);
    }
    return entry.name.endsWith(".astro") ? [path] : [];
  });
}

const markup = astroFiles(source).map((path) => ({ path, text: readFileSync(path, "utf8") }));

const literalKeys = markup.flatMap(({ path, text }) =>
  Array.from(text.matchAll(/\bt\("([^"]+)"/g), (match) => ({ path, key: match[1] })),
);

/** `t(`comparison.${row}.label`)` cubre toda una familia de claves, y cada hueco es un tramo sin punto. */
const keyPatterns = markup.flatMap(({ text }) =>
  Array.from(
    text.matchAll(/\bt\(`([^`]+)`/g),
    (match) => new RegExp(`^${match[1].replace(/\$\{[^}]+\}/g, "[^.]+")}$`),
  ),
);

const dictionaryKeys = Object.keys(es);

describe("landing markup", () => {
  it("only asks the dictionary for keys it has", () => {
    const missing = literalKeys
      .filter(({ key }) => !(key in es))
      .map(({ path, key }) => `${path}: ${key}`);
    expect(missing).toEqual([]);
  });

  it("leaves no key of the dictionary unused", () => {
    const asked = new Set(
      markup.flatMap(({ text }) => Array.from(text.matchAll(/"([\w.]+)"/g), (match) => match[1])),
    );
    const unused = dictionaryKeys.filter(
      (key) => !asked.has(key) && !keyPatterns.some((pattern) => pattern.test(key)),
    );
    expect(unused).toEqual([]);
  });
});
