import { describe, expect, it } from "vitest";
import type { Viewport } from "./pdf";
import { defaultBox, fitsInPage, movedBy, toPixels, toUserSpace } from "./signatureBox";

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

describe("el recuadro por omisión", () => {
  it("fits in the page it is drawn on", () => {
    const viewport = viewportOf(1);
    const box = defaultBox(viewport);

    expect(fitsInPage(box, viewport)).toBe(true);
  });

  it("scales with the zoom, so it looks the same size on the paper", () => {
    const small = defaultBox(viewportOf(1));
    const big = defaultBox(viewportOf(2));

    expect(big.width).toBeCloseTo(small.width * 2);
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
