/**
 * Las páginas del conjunto elegido donde el recuadro **no cabe** (ID-105).
 *
 * `correctPositionSignature` (`PdfUtil.java:607-632`, medido en
 * `docs/research/ancla-y-paginas-en-el-puente.md`) descarta en silencio, antes
 * de firmar, cualquier página del conjunto donde la esquina inferior izquierda
 * del recuadro no cabe — comparado contra el ancho y el alto de **cada**
 * página, no solo de la primera:
 *
 * ```java
 * if (pageSize.getWidth() <= signaturePosition.getLeft()
 *         || pageSize.getHeight() <= signaturePosition.getBottom()) {
 *     pagesList.remove(page);
 * }
 * ```
 *
 * Esta función es la mitad de interfaz de esa guardia: no impide nada —eso lo
 * hace `PadesBridge` al firmar—, solo la anticipa para que el diálogo de
 * páginas sin sello pueda avisar antes de que ocurra. Es pura y sin React,
 * como `signing/pageRange.ts`: la prueba a fondo va aparte de cualquier
 * componente.
 */

import type { UserSpaceRect } from "../viewer/signatureBox";

/** Una página del conjunto elegido, con su caja en espacio de usuario PDF. */
export interface SealedPage {
  /** Su número, 1-based, como los numera `pdf.js`. */
  number: number;
  /** `[x0, y0, x1, y1]` de su `MediaBox` (el `view` de `pdf.js`). */
  view: readonly [number, number, number, number];
}

/**
 * Las páginas de `pages` donde `rect` no cabe, en el mismo orden en que
 * llegan.
 */
export function pagesWithoutSeal(rect: UserSpaceRect, pages: readonly SealedPage[]): number[] {
  return pages
    .filter(({ view }) => {
      const width = view[2] - view[0];
      const height = view[3] - view[1];
      return width <= rect.x0 || height <= rect.y0;
    })
    .map(({ number }) => number);
}
