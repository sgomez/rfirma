import { describe, expect, it } from "vitest";
import type { Viewport } from "./pdf";
import {
  firstSealedPage,
  fitsInPage,
  movedBy,
  pageSetOf,
  resizedBy,
  sealedPages,
  sealing,
  sealsPage,
  standardBox,
  toPixels,
  toUserSpace,
  unsealing,
} from "./signatureBox";

/**
 * **Grada A** (`vitest`, carril rápido). ID-21 e ID-22.
 *
 * Lo que se comprueba aquí es que el frontal **no transforma coordenadas**:
 * llama a `convertToPdfPoint` y guarda lo que salga. La `T⁻¹` de la `/Rotate`
 * vive en `signing::placement` del backend y no tiene copia en TypeScript.
 */

/**
 * Un viewport de mentira con una transformación conocida: escala `scale` y
 * voltea el eje Y sobre una página de `height` puntos. No es `pdf.js`, es lo
 * justo para saber si el visor aplica algo **por encima** de lo que le devuelve
 * el viewport.
 */
function viewportOf(scale: number, width = 595, height = 842): Viewport {
  return {
    width: width * scale,
    height: height * scale,
    convertToPdfPoint: (x, y) => [x / scale, height - y / scale],
    convertToViewportPoint: (x, y) => [x * scale, (height - y) * scale],
  };
}

describe("el recuadro en espacio de usuario", () => {
  it("stores exactly what the viewport returned, with no transform of its own", () => {
    // Un viewport marcado: devuelve constantes. Si el visor le aplicara una
    // `T⁻¹` propia, estas constantes saldrían cambiadas.
    const marked: Viewport = {
      width: 100,
      height: 100,
      convertToPdfPoint: (x) => (x === 0 ? [11, 22] : [33, 44]),
      convertToViewportPoint: () => [0, 0],
    };

    expect(toUserSpace(marked, { x: 0, y: 0, width: 7, height: 7 })).toEqual({
      x0: 11,
      y0: 22,
      x1: 33,
      y1: 44,
    });
  });

  it("orders the corners, because the drag can go in any direction", () => {
    const upsideDown: Viewport = {
      width: 100,
      height: 100,
      convertToPdfPoint: (x) => (x === 0 ? [33, 44] : [11, 22]),
      convertToViewportPoint: () => [0, 0],
    };

    expect(toUserSpace(upsideDown, { x: 0, y: 0, width: 7, height: 7 })).toEqual({
      x0: 11,
      y0: 22,
      x1: 33,
      y1: 44,
    });
  });

  it("does not move the box over the document when the zoom changes", () => {
    // El fallo silencioso que este proyecto ya midió: guardado en píxeles, el
    // recuadro se queda clavado en la pantalla y se desplaza sobre el papel.
    const atOne = viewportOf(1);
    const placed = toUserSpace(atOne, { x: 60, y: 80, width: 200, height: 80 });

    for (const zoom of [0.5, 1.75, 3]) {
      const zoomed = viewportOf(zoom);
      const pixels = toPixels(zoomed, placed);

      // Los píxeles cambian con el zoom…
      expect(pixels.width).toBeCloseTo(200 * zoom);
      // …y volver a leerlos da el mismo sitio del documento.
      expect(toUserSpace(zoomed, pixels)).toEqual(placed);
    }
  });

  it("puts the box back where the drag left it", () => {
    const viewport = viewportOf(1.5);
    const pixels = { x: 30, y: 45, width: 300, height: 120 };

    expect(toPixels(viewport, toUserSpace(viewport, pixels))).toEqual(pixels);
  });
});

describe("la guardia de página", () => {
  const page = { width: 595, height: 842 };

  it("accepts a box that touches the edge", () => {
    expect(fitsInPage({ x: 0, y: 0, width: 595, height: 842 }, page)).toBe(true);
  });

  it("rejects a box hanging off any of the four sides", () => {
    expect(fitsInPage({ x: -1, y: 10, width: 100, height: 50 }, page)).toBe(false);
    expect(fitsInPage({ x: 10, y: -1, width: 100, height: 50 }, page)).toBe(false);
    expect(fitsInPage({ x: 500, y: 10, width: 100, height: 50 }, page)).toBe(false);
    expect(fitsInPage({ x: 10, y: 800, width: 100, height: 50 }, page)).toBe(false);
  });
});

describe("la posición estándar", () => {
  it("fits in the page it is drawn on", () => {
    const viewport = viewportOf(1);
    const box = standardBox(viewport);

    expect(fitsInPage(box, viewport)).toBe(true);
  });

  it("scales with the zoom, so it looks the same size on the paper", () => {
    const small = standardBox(viewportOf(1));
    const big = standardBox(viewportOf(2));

    expect(big.width).toBeCloseTo(small.width * 2);
  });

  /** ID-102: abajo a la derecha, a un 8 % del borde. */
  it("sits at the bottom right, an eighth of the page away from the edge", () => {
    const viewport = viewportOf(1);
    const box = standardBox(viewport);

    expect(viewport.width - (box.x + box.width)).toBeCloseTo(viewport.width * 0.08);
    expect(viewport.height - (box.y + box.height)).toBeCloseTo(viewport.height * 0.08);
  });
});

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

describe("los tiradores", () => {
  const rect = { x: 100, y: 100, width: 200, height: 80 };
  const min = { width: 120, height: 34 };

  it("moves the grabbed corner and leaves the opposite one where it was", () => {
    const grown = resizedBy(rect, "bottom-right", 40, 20, min, false);

    expect(grown).toEqual({ x: 100, y: 100, width: 240, height: 100 });
  });

  it("grows up and to the left from the top left corner", () => {
    const grown = resizedBy(rect, "top-left", -40, -20, min, false);

    expect(grown).toEqual({ x: 60, y: 80, width: 240, height: 100 });
  });

  /** ID-103: el gesto se para en el mínimo en vez de recortar el texto en silencio. */
  it("stops at the minimum size instead of shrinking past it", () => {
    const shrunk = resizedBy(rect, "bottom-right", -500, -500, min, false);

    expect(shrunk.width).toBe(min.width);
    expect(shrunk.height).toBe(min.height);
  });

  it("keeps the proportion with Shift held down", () => {
    const grown = resizedBy(rect, "bottom-right", 100, 0, min, true);

    expect(grown.width / grown.height).toBeCloseTo(rect.width / rect.height);
    expect(grown.width).toBeGreaterThan(rect.width);
  });

  /** Conservar la proporción no es una puerta trasera al tamaño ilegible. */
  it("still stops at the minimum with the proportion held", () => {
    const shrunk = resizedBy(rect, "top-left", 500, 500, min, true);

    expect(shrunk.width).toBeGreaterThanOrEqual(min.width);
    expect(shrunk.height).toBeGreaterThanOrEqual(min.height);
    expect(shrunk.width / shrunk.height).toBeCloseTo(rect.width / rect.height);
  });
});

describe("el desplazamiento del arrastre", () => {
  it("moves the box without resizing it", () => {
    expect(movedBy({ x: 10, y: 20, width: 100, height: 50 }, 5, -7)).toEqual({
      x: 15,
      y: 13,
      width: 100,
      height: 50,
    });
  });
});
