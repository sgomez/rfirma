import { describe, expect, it } from "vitest";
import { type PadesLowerLeft, pagesWithoutSeal } from "./unsealedPages";

// A4, en puntos: [0, 0, 595, 842].
const A4: readonly [number, number, number, number] = [0, 0, 595, 842];
// Un A5 apaisado, más pequeño en las dos dimensiones.
const SMALL: readonly [number, number, number, number] = [0, 0, 200, 150];

const cornerAt = (x: number, y: number): PadesLowerLeft => ({ x, y });

describe("pagesWithoutSeal", () => {
  // Refleja PdfUtil.correctPositionSignature: se descarta la página cuando la
  // esquina inferior izquierda del recuadro —ya en puntos PAdES— no cabe en
  // su ancho o su alto.
  it("keeps a page whose size fits the box's bottom-left corner", () => {
    const lowerLeft = cornerAt(100, 100);

    expect(pagesWithoutSeal(lowerLeft, [{ number: 1, view: A4 }])).toEqual([]);
  });

  it("drops a page too narrow for the box's left edge", () => {
    const lowerLeft = cornerAt(250, 50);

    expect(pagesWithoutSeal(lowerLeft, [{ number: 1, view: SMALL }])).toEqual([1]);
  });

  it("drops a page too short for the box's bottom edge", () => {
    const lowerLeft = cornerAt(50, 180);

    expect(pagesWithoutSeal(lowerLeft, [{ number: 1, view: SMALL }])).toEqual([1]);
  });

  it("names only the pages that fall, keeping the chosen order", () => {
    const lowerLeft = cornerAt(250, 50);

    expect(
      pagesWithoutSeal(lowerLeft, [
        { number: 1, view: A4 },
        { number: 2, view: SMALL },
        { number: 3, view: A4 },
        { number: 4, view: SMALL },
      ]),
    ).toEqual([2, 4]);
  });

  it("names none when the box fits every chosen page", () => {
    const lowerLeft = cornerAt(10, 10);

    expect(
      pagesWithoutSeal(lowerLeft, [
        { number: 1, view: A4 },
        { number: 2, view: SMALL },
      ]),
    ).toEqual([]);
  });

  // El hallazgo: con la página rotada, la esquina PAdES que compara
  // `correctPositionSignature` no coincide con la de espacio de usuario —esta
  // función ya no ve espacio de usuario en absoluto, y por eso no puede
  // volver a confundirlos. El caso concreto (rect en (250,50)-(450,100),
  // A4, `/Rotate 90` → esquina PAdES en (50,145)) se prueba del lado del
  // backend, en `commands::pades_lower_left`, que es quien hace la
  // conversión.
  it("compares the PAdES corner it is given, not a user-space one", () => {
    // La misma página A4 cabría con la esquina de espacio de usuario
    // (250,50), pero no con la esquina PAdES a la que convierte una
    // `/Rotate 90` — (50,145) según `commands::pades_lower_left`.
    const padesLowerLeftAfterRotation = cornerAt(50, 145);

    expect(pagesWithoutSeal(padesLowerLeftAfterRotation, [{ number: 1, view: A4 }])).toEqual([]);
  });
});
