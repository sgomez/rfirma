import type { Viewport } from "./pdf";

/**
 * El recuadro de la firma visible: dónde se guarda y cómo se pinta.
 *
 * **Se guarda en espacio de usuario PDF, nunca en píxeles** (ID-21). Los
 * píxeles se derivan del viewport en cada pintada, así que el zoom es
 * puramente visual: acercarse no mueve la firma. Guardado en píxeles, el
 * recuadro se queda clavado en la pantalla y se desplaza sobre el documento sin
 * que nadie lo toque —el fallo silencioso que mide
 * `docs/research/coordenadas-recuadro-pades.md`—.
 *
 * Aquí acaba lo que sabe el frontal. Convertir este rectángulo a los
 * `extraParams` de posición de PAdES es un **segundo** paso, la `T⁻¹` de la
 * `/Rotate` que iText aplica al cerrar el documento, y vive en
 * `signing::placement` del backend (`Page::signature_box`). No hay ni debe
 * haber una copia en TypeScript: si te encuentras escribiendo una tabla por
 * rotación en este directorio, estás duplicando ese módulo.
 */

/** El recuadro en espacio de usuario PDF, con las esquinas ya ordenadas. */
export interface UserSpaceRect {
  /** Esquina inferior izquierda, eje X. */
  x0: number;
  /** Esquina inferior izquierda, eje Y. */
  y0: number;
  /** Esquina superior derecha, eje X. */
  x1: number;
  /** Esquina superior derecha, eje Y. */
  y1: number;
}

/**
 * El recuadro colocado: en qué página y dónde.
 *
 * Es lo único que el visor entrega hacia arriba, y el mismo dato que espera
 * `signing::placement::Page` en el backend: `page` es **1-based**, como lo
 * numera `pdf.js` y como lo cuenta `signaturePage`.
 */
export interface SignaturePlacement {
  page: number;
  rect: UserSpaceRect;
}

/** El recuadro en píxeles del lienzo. Dato **de paso**: se pinta y se tira. */
export interface PixelRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** El tamaño del lienzo sobre el que se arrastra. */
export interface PageSize {
  width: number;
  height: number;
}

/**
 * Píxeles → espacio de usuario, que es el paso 1 del ID-21 tal cual lo hace
 * `pdf.js`. Las dos esquinas se ordenan porque se arrastra en cualquier
 * dirección y el recuadro es el mismo.
 */
export function toUserSpace(viewport: Viewport, pixels: PixelRect): UserSpaceRect {
  const [ax, ay] = viewport.convertToPdfPoint(pixels.x, pixels.y);
  const [bx, by] = viewport.convertToPdfPoint(pixels.x + pixels.width, pixels.y + pixels.height);
  return {
    x0: Math.min(ax, bx),
    y0: Math.min(ay, by),
    x1: Math.max(ax, bx),
    y1: Math.max(ay, by),
  };
}

/** Espacio de usuario → píxeles, que es lo que se pinta a cada zoom. */
export function toPixels(viewport: Viewport, rect: UserSpaceRect): PixelRect {
  const [ax, ay] = viewport.convertToViewportPoint(rect.x0, rect.y0);
  const [bx, by] = viewport.convertToViewportPoint(rect.x1, rect.y1);
  const x = Math.min(ax, bx);
  const y = Math.min(ay, by);
  return { x, y, width: Math.max(ax, bx) - x, height: Math.max(ay, by) - y };
}

/** El recuadro desplazado por el arrastre. No cambia de tamaño. */
export function movedBy(rect: PixelRect, dx: number, dy: number): PixelRect {
  return { ...rect, x: rect.x + dx, y: rect.y + dy };
}

/**
 * ¿Cabe entero en la página? (ID-22).
 *
 * Es la mitad de interfaz de la guardia: aquí se impide **soltarlo** fuera, con
 * aviso, en píxeles del lienzo, que es lo que la persona ve. La mitad
 * autoritativa está en `signing::placement`, justo antes de firmar, porque un
 * recuadro que se sale iText lo recorta en silencio y la firma sale válida
 * igual, con la rúbrica de 13 pt de ancho en vez de los 200 que se dibujaron.
 */
export function fitsInPage(rect: PixelRect, page: PageSize): boolean {
  return (
    rect.x >= 0 &&
    rect.y >= 0 &&
    rect.x + rect.width <= page.width &&
    rect.y + rect.height <= page.height
  );
}

/**
 * Dónde aparece el recuadro la primera vez.
 *
 * Abajo a la izquierda, que es donde suele ir la firma en un documento
 * administrativo, y en proporción a la página para que el zoom no lo cambie de
 * tamaño sobre el papel. A partir de ahí se arrastra: la posición es libre y no
 * hay rejilla (ID-26).
 */
export function defaultBox(page: PageSize): PixelRect {
  const width = page.width * 0.34;
  const height = page.height * 0.095;
  const margin = page.height * 0.08;
  return { x: page.width * 0.08, y: page.height - height - margin, width, height };
}
