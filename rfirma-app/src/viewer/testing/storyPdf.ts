//! Un PDF de mentira para las historias del visor: hojas A4 en blanco con renglones, sin `pdf.js` ni contexto `2d`.

import type { UserSpaceRect } from "../../placement/pageSets";
import type { PdfDocument, PdfPage, Viewport } from "../pdf";

const A4 = { width: 595, height: 842 };

function viewportAt(scale: number): Viewport {
  return {
    width: A4.width * scale,
    height: A4.height * scale,
    convertToPdfPoint: (x, y) => [x / scale, A4.height - y / scale],
    convertToViewportPoint: (x, y) => [x * scale, (A4.height - y) * scale],
  };
}

const RULES = "repeating-linear-gradient(#fff 0 22px, #e4e4e7 22px 23px)";

function stampImage({ x0, y0, x1, y1 }: UserSpaceRect): string {
  const [width, height] = [x1 - x0, y1 - y0];
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${A4.width} ${A4.height}">` +
    `<rect x="${x0}" y="${A4.height - y1}" width="${width}" height="${height}" fill="#fff" stroke="#334155"/>` +
    `<text x="${x0 + 8}" y="${A4.height - y1 + 22}" font-family="sans-serif" font-size="11" font-weight="700" fill="#0f172a">Firmado por</text>` +
    `<text x="${x0 + 8}" y="${A4.height - y1 + 38}" font-family="sans-serif" font-size="11" fill="#0f172a">PERSONA DE PRUEBAS</text>` +
    "</svg>";
  return `url("data:image/svg+xml,${encodeURIComponent(svg)}") 0 0 / 100% 100% no-repeat`;
}

function pageOf(number: number, background = RULES): PdfPage {
  return {
    number,
    rotate: 0,
    view: [0, 0, A4.width, A4.height],
    getViewport: ({ scale }) => viewportAt(scale),
    render: ({ canvas }) => {
      canvas.style.background = background;
      return { promise: Promise.resolve(), cancel: () => {} };
    },
  };
}

/** Un documento de `pageCount` hojas que se pinta al instante. */
export function storyPdf(pageCount = 3): PdfDocument {
  return { pageCount, getPage: (number) => Promise.resolve(pageOf(number)) };
}

/** El mismo documento con el sello ya estampado en `rect`, en la página 1. */
export function storyStampedPdf(rect: UserSpaceRect, pageCount = 3): PdfDocument {
  return {
    pageCount,
    getPage: (number) =>
      Promise.resolve(pageOf(number, number === 1 ? `${stampImage(rect)}, ${RULES}` : RULES)),
  };
}
