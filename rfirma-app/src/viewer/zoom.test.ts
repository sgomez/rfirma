import { describe, expect, it } from "vitest";
import {
  anchoredScroll,
  bitmapScale,
  DEFAULT_ZOOM,
  fitScale,
  MAX_BITMAP_SCALE,
  pinchedZoom,
  steppedZoom,
  typedZoom,
  ZOOM_MAX,
  ZOOM_MIN,
} from "./zoom";

/** **Grada A** (`vitest`, carril rápido). Sub-issue #173. */

const A4 = { width: 595, height: 842 };

describe("el rango del zoom", () => {
  it("goes from a quarter to four times, continuously", () => {
    expect(ZOOM_MIN).toBe(0.25);
    expect(ZOOM_MAX).toBe(4);
  });

  it("trips over the seven steps going up, and reaches the ceiling from the last one", () => {
    expect(steppedZoom(1, 1)).toBe(1.25);
    expect(steppedZoom(1.1, 1)).toBe(1.25);
    expect(steppedZoom(3, 1)).toBe(ZOOM_MAX);
  });

  it("trips over the seven steps going down, and reaches the floor from the first one", () => {
    expect(steppedZoom(1, -1)).toBe(0.75);
    expect(steppedZoom(0.9, -1)).toBe(0.75);
    expect(steppedZoom(0.5, -1)).toBe(ZOOM_MIN);
  });
});

describe("el pellizco y la rueda con Ctrl", () => {
  it("multiplies instead of adding, so every notch feels the same", () => {
    const inwards = pinchedZoom(1, -100);
    const twice = pinchedZoom(inwards, -100);

    expect(inwards).toBeGreaterThan(1);
    expect(twice / inwards).toBeCloseTo(inwards / 1, 6);
  });

  it("scrolling the other way shrinks, and the two cancel out", () => {
    expect(pinchedZoom(pinchedZoom(1, -120), 120)).toBeCloseTo(1, 6);
  });

  it("never leaves the range, however hard the gesture pushes", () => {
    expect(pinchedZoom(1, -100000)).toBe(ZOOM_MAX);
    expect(pinchedZoom(1, 100000)).toBe(ZOOM_MIN);
  });
});

describe("el ancla del puntero", () => {
  it("keeps the document point under the pointer where it was", () => {
    // El punto del documento bajo el puntero está a 300 px del origen del
    // lienzo (100 de desplazamiento + 200 dentro de la parte visible). Al
    // doblar la escala pasa a estar a 600, y para que siga bajo el puntero el
    // desplazamiento tiene que ser 600 - 200.
    const moved = anchoredScroll({ left: 100, top: 40 }, { x: 200, y: 60 }, 2);

    expect(moved).toEqual({ left: 400, top: 140 });
  });

  it("does not ask for a negative scroll when zooming out at the origin", () => {
    expect(anchoredScroll({ left: 0, top: 0 }, { x: 200, y: 60 }, 0.5)).toEqual({
      left: 0,
      top: 0,
    });
  });
});

describe("el porcentaje tecleado", () => {
  it("takes what was typed, with or without the sign", () => {
    expect(typedZoom("150")).toBe(1.5);
    expect(typedZoom("150 %")).toBe(1.5);
    expect(typedZoom("87,5%")).toBe(0.875);
  });

  it("clips to the range instead of refusing the number", () => {
    expect(typedZoom("1000")).toBe(ZOOM_MAX);
    expect(typedZoom("1")).toBe(ZOOM_MIN);
  });

  it("says no to what is not a percentage", () => {
    expect(typedZoom("")).toBeNull();
    expect(typedZoom("ajustar")).toBeNull();
    expect(typedZoom("0")).toBeNull();
  });
});

describe("el ajuste de partida", () => {
  it("opens a fresh document fitted to the whole page, not a free percentage (ID-117 enmendado)", () => {
    expect(DEFAULT_ZOOM).toEqual({ kind: "fit-page" });
  });
});

describe("«ajustar» como modo", () => {
  it("fits the width of the sheet in the surface, with room on the sides", () => {
    const scale = fitScale({ kind: "fit-width" }, { width: 800, height: 400 }, A4);

    expect(scale).toBeCloseTo((800 * 0.92) / A4.width, 6);
  });

  it("fits the whole page, so the tighter of the two axes wins", () => {
    const scale = fitScale({ kind: "fit-page" }, { width: 800, height: 400 }, A4);

    expect(scale).toBeCloseTo((400 * 0.92) / A4.height, 6);
  });

  it("fits a landscape page too, where 'fit width' would leave part of it out (ID-117 enmendado)", () => {
    const landscape = { width: A4.height, height: A4.width };
    const surface = { width: 800, height: 400 };
    const scale = fitScale({ kind: "fit-page" }, surface, landscape);

    // Contra una superficie de 800×400, el alto de la hoja apaisada (595) es
    // el eje que aprieta: «ajustar al ancho» daría un porcentaje mayor —el
    // que sale de los 842 de ancho— y dejaría el alto fuera.
    expect(scale).toBeCloseTo((surface.height * 0.92) / landscape.height, 6);
    const byWidth = fitScale({ kind: "fit-width" }, surface, landscape);
    expect(byWidth).not.toBeNull();
    expect(scale).toBeLessThan(byWidth as number);
  });

  it("clips a fit that would fall outside the range", () => {
    expect(fitScale({ kind: "fit-width" }, { width: 8000, height: 8000 }, A4)).toBe(ZOOM_MAX);
    expect(fitScale({ kind: "fit-width" }, { width: 10, height: 10 }, A4)).toBe(ZOOM_MIN);
  });

  it("has nothing to recompute for a zoom fixed by hand, nor without measurements", () => {
    expect(fitScale({ kind: "free", value: 2 }, { width: 800, height: 400 }, A4)).toBeNull();
    expect(fitScale({ kind: "fit-width" }, null, A4)).toBeNull();
    expect(fitScale({ kind: "fit-width" }, { width: 800, height: 400 }, null)).toBeNull();
    expect(fitScale({ kind: "fit-width" }, { width: 0, height: 0 }, A4)).toBeNull();
  });
});

describe("el tope del mapa de bits", () => {
  it("paints at devicePixelRatio while there is room under the ceiling", () => {
    expect(bitmapScale(1, 2)).toBe(2);
    expect(bitmapScale(1.5, 2)).toBe(3);
  });

  it("caps at four times, so a page at 400 % on a 2x screen is not eight", () => {
    expect(bitmapScale(4, 2)).toBe(MAX_BITMAP_SCALE);
    expect(bitmapScale(3, 2)).toBe(MAX_BITMAP_SCALE);
  });

  it("never paints below the zoom, whatever the screen says", () => {
    expect(bitmapScale(2, 0.5)).toBe(2);
  });
});
