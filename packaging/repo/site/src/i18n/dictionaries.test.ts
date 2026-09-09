import { describe, expect, it } from "vitest";
import { dictionaries, invariantKeys, locales } from "./index";
import { es } from "./es";

const keys = Object.keys(es) as (keyof typeof es)[];
const translated = locales.filter((locale) => locale !== "es");

describe("landing dictionaries", () => {
  it("has the same key list in the five languages", () => {
    for (const locale of locales) {
      expect(Object.keys(dictionaries[locale]).sort()).toEqual([...keys].sort());
    }
  });

  it("has no empty value in any language", () => {
    for (const locale of locales) {
      const empty = keys.filter((key) => dictionaries[locale][key].trim() === "");
      expect(empty).toEqual([]);
    }
  });

  it("only repeats the Spanish text on keys declared invariant", () => {
    const repeated = translated.flatMap((locale) =>
      keys
        .filter((key) => dictionaries[locale][key] === es[key] && !invariantKeys.includes(key))
        .map((key) => `${locale}:${key}`),
    );
    expect(repeated).toEqual([]);
  });

  it("declares no invariant key that does not exist", () => {
    for (const key of invariantKeys) {
      expect(keys).toContain(key);
    }
  });
});
