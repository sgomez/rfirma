import { describe, expect, it } from "vitest";
import type { UserSpaceRect } from "../viewer/signatureBox";
import { pagesWithoutSeal } from "./unsealedPages";

// A4, en puntos: [0, 0, 595, 842].
const A4: readonly [number, number, number, number] = [0, 0, 595, 842];
// Un A5 apaisado, más pequeño en las dos dimensiones.
const SMALL: readonly [number, number, number, number] = [0, 0, 200, 150];

const boxAt = (x0: number, y0: number, x1: number, y1: number): UserSpaceRect => ({
  x0,
  y0,
  x1,
  y1,
});

describe("pagesWithoutSeal", () => {
  // Refleja PdfUtil.correctPositionSignature: se descarta la página cuando la
  // esquina inferior izquierda del recuadro no cabe en su ancho o su alto.
  it("keeps a page whose size fits the box's bottom-left corner", () => {
    const rect = boxAt(100, 100, 300, 200);

    expect(pagesWithoutSeal(rect, [{ number: 1, view: A4 }])).toEqual([]);
  });

  it("drops a page too narrow for the box's left edge", () => {
    const rect = boxAt(250, 50, 450, 100);

    expect(pagesWithoutSeal(rect, [{ number: 1, view: SMALL }])).toEqual([1]);
  });

  it("drops a page too short for the box's bottom edge", () => {
    const rect = boxAt(50, 180, 250, 220);

    expect(pagesWithoutSeal(rect, [{ number: 1, view: SMALL }])).toEqual([1]);
  });

  it("names only the pages that fall, keeping the chosen order", () => {
    const rect = boxAt(250, 50, 450, 100);

    expect(
      pagesWithoutSeal(rect, [
        { number: 1, view: A4 },
        { number: 2, view: SMALL },
        { number: 3, view: A4 },
        { number: 4, view: SMALL },
      ]),
    ).toEqual([2, 4]);
  });

  it("names none when the box fits every chosen page", () => {
    const rect = boxAt(10, 10, 60, 40);

    expect(
      pagesWithoutSeal(rect, [
        { number: 1, view: A4 },
        { number: 2, view: SMALL },
      ]),
    ).toEqual([]);
  });
});
