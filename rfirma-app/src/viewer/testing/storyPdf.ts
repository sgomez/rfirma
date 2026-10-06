//! Un PDF de mentira para las historias del visor: hojas A4 en blanco con renglones, sin `pdf.js` ni contexto `2d`.

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

function pageOf(number: number): PdfPage {
  return {
    number,
    rotate: 0,
    view: [0, 0, A4.width, A4.height],
    getViewport: ({ scale }) => viewportAt(scale),
    render: ({ canvas }) => {
      canvas.style.background = "repeating-linear-gradient(#fff 0 22px, #e4e4e7 22px 23px)";
      return { promise: Promise.resolve(), cancel: () => {} };
    },
  };
}

/** Un documento de `pageCount` hojas que se pinta al instante. */
export function storyPdf(pageCount = 3): PdfDocument {
  return { pageCount, getPage: (number) => Promise.resolve(pageOf(number)) };
}
