import { describe, expect, it } from "vitest";
import { unsealing } from "../viewer/signatureBox";
import { formatPageRange, parsePageRange } from "./pageRange";

/** El recuadro no importa aquí: lo que se prueba es el conjunto. */
const rect = { x0: 0, y0: 0, x1: 10, y1: 10 };

// Grada A: aritmética pura, sin componente ni catálogo (TD-29).
describe("parsePageRange", () => {
  it("reads the everyday print format", () => {
    expect(parsePageRange("1,2-3,10-20", 27)).toEqual({
      ok: true,
      pages: { only: [1, 2, 3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20] },
    });
  });

  it("accepts spaces around the entries, but not inside a range", () => {
    expect(parsePageRange(" 1 , 2-3 ", 27)).toEqual({ ok: true, pages: { only: [1, 2, 3] } });
    expect(parsePageRange("2 - 3", 27)).toEqual({
      ok: false,
      error: { kind: "malformed", entry: "2 - 3" },
    });
  });

  it("leaves the set empty on an empty field, which is not a failure", () => {
    expect(parsePageRange("", 27)).toEqual({ ok: true, pages: null });
    expect(parsePageRange("   ", 27)).toEqual({ ok: true, pages: null });
  });

  it("merges overlaps and duplicates instead of repeating a page", () => {
    expect(parsePageRange("1-3,2-5,3,3", 27)).toEqual({
      ok: true,
      pages: { only: [1, 2, 3, 4, 5] },
    });
  });

  it("orders the set even when it is typed backwards", () => {
    expect(parsePageRange("10,2", 27)).toEqual({ ok: true, pages: { only: [2, 10] } });
  });

  it("refuses a page the document does not have, naming the highest one written", () => {
    expect(parsePageRange("99", 27)).toEqual({
      ok: false,
      error: { kind: "beyond", page: 99, pageCount: 27 },
    });
    expect(parsePageRange("10-40", 27)).toEqual({
      ok: false,
      error: { kind: "beyond", page: 40, pageCount: 27 },
    });
  });

  it("refuses a range that runs backwards", () => {
    expect(parsePageRange("3-1", 27)).toEqual({
      ok: false,
      error: { kind: "reversed", entry: "3-1" },
    });
  });

  it("refuses page zero, because the first one is the 1", () => {
    expect(parsePageRange("0", 27)).toEqual({ ok: false, error: { kind: "zero" } });
    expect(parsePageRange("0-3", 27)).toEqual({ ok: false, error: { kind: "zero" } });
  });

  it("refuses a separator it does not understand instead of reading half of it", () => {
    expect(parsePageRange("1;2", 27)).toEqual({
      ok: false,
      error: { kind: "malformed", entry: "1;2" },
    });
    expect(parsePageRange("1,2;3", 27)).toEqual({
      ok: false,
      error: { kind: "malformed", entry: "2;3" },
    });
  });

  it("refuses the negative ranges of AutoFirma, which are not this syntax", () => {
    expect(parsePageRange("-3--1", 27)).toEqual({
      ok: false,
      error: { kind: "malformed", entry: "-3--1" },
    });
  });

  it("stops at the first entry it cannot resolve, and does not pile up complaints", () => {
    expect(parsePageRange("3-1,99", 27)).toEqual({
      ok: false,
      error: { kind: "reversed", entry: "3-1" },
    });
  });
});

describe("formatPageRange", () => {
  it("compresses runs and leaves singles alone", () => {
    expect(formatPageRange({ only: [3, 10, 11, 13, 14, 15] }, 27)).toBe("3,10-11,13-15");
  });

  it("writes «all» as the whole document", () => {
    expect(formatPageRange("all", 4)).toBe("1-4");
  });

  it("compresses a pair as a range, not as two numbers", () => {
    expect(formatPageRange({ only: [10, 11] }, 27)).toBe("10-11");
  });

  it("rewrites the field when a page is unsealed from the viewer (ID-99)", () => {
    const parsed = parsePageRange("3,10-20", 27);
    if (!parsed.ok || parsed.pages === null) throw new Error("3,10-20 has to parse");

    const left = unsealing({ rect, pages: parsed.pages }, 12, 27);

    expect(left && formatPageRange(left.pages, 27)).toBe("3,10-11,13-20");
  });

  it("round-trips whatever it wrote", () => {
    const written = formatPageRange({ only: [1, 2, 3, 10, 11, 12] }, 27);

    expect(parsePageRange(written, 27)).toEqual({
      ok: true,
      pages: { only: [1, 2, 3, 10, 11, 12] },
    });
  });
});
