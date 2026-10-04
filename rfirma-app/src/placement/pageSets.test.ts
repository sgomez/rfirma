import { describe, expect, it } from "vitest";
import {
  activating,
  firstSealedPage,
  NO_PAGE_SETS,
  type PageSets,
  pageSetOf,
  pagesOf,
  placementOf,
  sealedPages,
  sealing,
  sealsPage,
  storing,
  unsealing,
} from "./pageSets";

describe("el conjunto de páginas", () => {
  const rect = { x0: 50, y0: 60, x1: 250, y1: 140 };

  it("seals every page of the set and no other", () => {
    expect(sealsPage({ only: [1, 3] }, 1)).toBe(true);
    expect(sealsPage({ only: [1, 3] }, 2)).toBe(false);
    expect(sealsPage("all", 27)).toBe(true);
  });

  /** El ID-91 del backend, en el lado de la ventana: «todas» y la lista completa son lo mismo. */
  it("names the same pages as the full list when it says all", () => {
    expect(sealedPages("all", 3)).toEqual([1, 2, 3]);
    expect(sealedPages({ only: [3, 1] }, 3)).toEqual([3, 1]);
  });

  it("orders and deduplicates what it is given", () => {
    expect(pageSetOf([3, 1, 3])).toEqual({ only: [1, 3] });
  });

  /** ID-92: un conjunto vacío no es una colocación, es la ausencia de una. */
  it("is nothing at all when no page is left", () => {
    expect(pageSetOf([])).toBeNull();
  });

  it("opens on the first page of the set", () => {
    expect(firstSealedPage(null)).toBeNull();
    expect(firstSealedPage({ rect, pages: { only: [3, 7] } })).toBe(3);
    expect(firstSealedPage({ rect, pages: "all" })).toBe(1);
  });

  it("adds a page without touching the rectangle", () => {
    expect(sealing({ rect, pages: { only: [3] } }, 7)).toEqual({ rect, pages: { only: [3, 7] } });
  });

  it("changes nothing when every page is already sealed", () => {
    const all = { rect, pages: "all" } as const;

    expect(sealing(all, 7)).toBe(all);
  });

  it("spells out the rest of the pages when one is taken off all of them", () => {
    expect(unsealing({ rect, pages: "all" }, 2, 3)).toEqual({ rect, pages: { only: [1, 3] } });
  });

  /** ID-92: quitar la última página devuelve al estado del PDF recién abierto. */
  it("takes the whole placement away with the last page of the set", () => {
    expect(unsealing({ rect, pages: { only: [3] } }, 3, 10)).toBeNull();
  });
});

/**
 * El conjunto propio de cada opción (#188).
 *
 * Las tres funciones son la respuesta entera al fallo: sin ellas, la opción
 * activa reescribía la que dejabas y `Solo 1 página` acababa nombrando tres.
 */
describe("los tres conjuntos del bloque «Colocación»", () => {
  const rect = { x0: 100, y0: 100, x1: 300, y1: 180 };
  const sets: PageSets = { single: 3, these: { only: [2, 5] } };

  it("reads the set of the option in charge, and only that one", () => {
    expect(pagesOf(sets, "single")).toEqual({ only: [3] });
    expect(pagesOf(sets, "these")).toEqual({ only: [2, 5] });
    expect(pagesOf(sets, "all")).toBe("all");
  });

  it("has no placement without a box, and none for an option that names no page", () => {
    expect(placementOf(null, sets, "single")).toBeNull();
    // ID-92 preguntado por opción: el recuadro está puesto, pero «Estas
    // páginas» no nombra ninguna, así que con ella delante no hay colocación.
    expect(placementOf(rect, { single: 3, these: null }, "these")).toBeNull();
    expect(placementOf(rect, sets, "these")).toEqual({ rect, pages: { only: [2, 5] } });
  });

  it("stores in the option in charge and leaves the other two alone", () => {
    expect(storing(sets, "single", { only: [7] }, 8)).toEqual({
      single: 7,
      these: { only: [2, 5] },
    });
    expect(storing(sets, "these", { only: [1, 4] }, 8)).toEqual({
      single: 3,
      these: { only: [1, 4] },
    });
    // «Todas» no tiene conjunto que guardar: es la palabra, siempre la misma.
    expect(storing(sets, "all", "all", 8)).toBe(sets);
  });

  /** Una página es lo único que esa opción puede nombrar, venga lo que venga. */
  it("keeps a single page under «one page only», never a set", () => {
    expect(storing(sets, "single", { only: [4, 9] }, 12).single).toBe(4);
    expect(storing(sets, "single", "all", 12).single).toBe(1);
    expect(storing(sets, "single", null, 12).single).toBeNull();
  });

  it("seeds an option the first time it is chosen, and never again", () => {
    // Se estrena: hereda la 3 del conjunto que venía, que es lo que pide la
    // ficha —«el campo arranca con esa misma página escrita»—.
    expect(activating(NO_PAGE_SETS, "these", { only: [3] }, 8, 1).these).toEqual({ only: [3] });
    // Ya tenía el suyo: vuelve lo suyo, y no lo que dejó la opción anterior.
    expect(activating(sets, "these", { only: [3] }, 8, 1).these).toEqual({ only: [2, 5] });
    expect(activating(sets, "single", "all", 8, 1).single).toBe(3);
  });

  /** Sin nada colocado en ninguna parte, la única respuesta es la que se mira. */
  it("falls back to the page on screen when nothing has ever been placed", () => {
    expect(activating(NO_PAGE_SETS, "single", null, 8, 6).single).toBe(6);
  });
});
