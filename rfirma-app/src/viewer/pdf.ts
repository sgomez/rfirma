/**
 * La frontera con `pdf.js`, escrita como puerto.
 *
 * `pdf.js` es imperativo —pinta sobre un lienzo y devuelve tareas que se
 * cancelan— y eso no se puede probar en `jsdom`, donde no hay contexto `2d`.
 * De ahí este puerto: el visor programa pintadas contra estas interfaces, la
 * librería de verdad entra por [`pdfjsLoader`](./pdfjsLoader.ts) y las pruebas
 * enchufan un doble. Solo se declara lo que el visor usa, no la API entera.
 *
 * Los nombres son los de `pdf.js` a propósito (`getViewport`, `render`,
 * `convertToPdfPoint`): el adaptador tiene que ser una línea por método, y un
 * vocabulario propio aquí solo añadiría una traducción que revisar.
 */

/** La página proyectada a píxeles del lienzo, a una escala concreta. */
export interface Viewport {
  /** Ancho del lienzo, en píxeles, ya multiplicado por la escala. */
  readonly width: number;
  /** Alto del lienzo, en píxeles, ya multiplicado por la escala. */
  readonly height: number;
  /**
   * Píxeles del lienzo → **espacio de usuario PDF**. Es el paso 1 del ID-21, y
   * el único punto del frontal donde se convierten coordenadas: invierte la
   * matriz del viewport, que deshace de golpe la escala, el volteo del eje Y,
   * la `/Rotate` y el origen de la MediaBox.
   */
  convertToPdfPoint(x: number, y: number): [number, number];
  /** Espacio de usuario PDF → píxeles del lienzo. Lo inverso del anterior. */
  convertToViewportPoint(x: number, y: number): [number, number];
}

/**
 * Una pintada en vuelo.
 *
 * Se cancela, y cancelarla hace que su `promise` **rechace** con un error
 * llamado `RenderingCancelledException`. Quien la espere tiene que contar con
 * ello; de eso se encarga [`createRenderQueue`](./renderQueue.ts).
 */
export interface RenderTask {
  readonly promise: Promise<void>;
  cancel(): void;
}

/** Una página del documento. */
export interface PdfPage {
  /** Su número, **1-based**, como los numera `pdf.js` y como los cuenta PAdES. */
  readonly number: number;
  /** La `/Rotate` de la página, en grados. */
  readonly rotate: number;
  /**
   * La caja de la página, `[x0, y0, x1, y1]` en espacio de usuario: el `view`
   * de `pdf.js`, que es la `CropBox` recortada a la `MediaBox`.
   *
   * Está aquí porque la conversión del recuadro a puntos PAdES la hace el
   * backend (`signing::placement`) y **necesita la caja y la rotación**, y
   * quien tiene el PDF abierto es este visor. Releerlas en Rust exigiría un
   * analizador de PDF —que este proyecto no tiene, y a propósito— para acabar
   * en una segunda opinión sobre la misma página.
   */
  readonly view: readonly [number, number, number, number];
  getViewport(options: { scale: number }): Viewport;
  /**
   * Pinta la página sobre el lienzo.
   *
   * Recibe el `<canvas>` y no su contexto `2d` para que el visor no tenga que
   * pedir uno: en `jsdom` `getContext("2d")` devuelve `null` y toda la columna
   * quedaría fuera de la grada A. El contexto lo saca el adaptador.
   */
  render(options: { canvas: HTMLCanvasElement; viewport: Viewport }): RenderTask;
}

/** El documento abierto. */
export interface PdfDocument {
  readonly pageCount: number;
  /** La página `number`, 1-based. */
  getPage(number: number): Promise<PdfPage>;
}

/**
 * Quien abre un PDF.
 *
 * Los bytes llegan ya leídos porque bajo el sandbox el documento entra por el
 * portal y la aplicación nunca conoce su ruta original: pasarle una URL a
 * `pdf.js` sería el segundo camino de entrada que el ADR-0004 no admite.
 */
export interface PdfLoader {
  load(bytes: Uint8Array): Promise<PdfDocument>;
}
