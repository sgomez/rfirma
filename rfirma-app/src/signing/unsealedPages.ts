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
 * `signaturePosition` está en **puntos PAdES**, no en el espacio de usuario
 * PDF donde vive `placement.rect`: son los dos espacios distintos que describe
 * `signing::placement` en el backend, y la esquina inferior izquierda solo
 * coincide entre ambos cuando la `/Rotate` de la página es 0. Con 90, 180 o
 * 270 no coincide, y comparar `placement.rect` aquí se equivoca en los dos
 * sentidos. Por eso esta función recibe la esquina **ya convertida**, pedida
 * al backend con la orden `pades_lower_left` — la misma conversión (`T⁻¹`)
 * que arma la orden de verdad, y sin una segunda copia de esa tabla en
 * TypeScript.
 *
 * Esta función es la mitad de interfaz de esa guardia: no impide nada —eso lo
 * hace `PadesBridge` al firmar—, solo la anticipa para que el diálogo de
 * páginas sin sello pueda avisar antes de que ocurra. Es pura y sin React,
 * como `signing/pageRange.ts`: la prueba a fondo va aparte de cualquier
 * componente.
 */

/** Una página del conjunto elegido, con su caja en espacio de usuario PDF. */
export interface SealedPage {
  /** Su número, 1-based, como los numera `pdf.js`. */
  number: number;
  /** `[x0, y0, x1, y1]` de su `MediaBox` (el `view` de `pdf.js`). */
  view: readonly [number, number, number, number];
}

/**
 * La esquina inferior izquierda del recuadro, ya en **puntos PAdES**: la que
 * de verdad compara `correctPositionSignature`, no la de espacio de usuario
 * PDF. La pide `pades_lower_left` al backend.
 */
export interface PadesLowerLeft {
  x: number;
  y: number;
}

/**
 * Las páginas de `pages` donde `lowerLeft` no cabe, en el mismo orden en que
 * llegan.
 */
export function pagesWithoutSeal(
  lowerLeft: PadesLowerLeft,
  pages: readonly SealedPage[],
): number[] {
  return pages
    .filter(({ view }) => {
      const width = view[2] - view[0];
      const height = view[3] - view[1];
      return width <= lowerLeft.x || height <= lowerLeft.y;
    })
    .map(({ number }) => number);
}
