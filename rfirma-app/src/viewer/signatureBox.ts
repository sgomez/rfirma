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
 * En qué páginas se estampa el recuadro.
 *
 * Cruza tal cual lo lee `signing::placement::PageSet`: la palabra `"all"`, o el
 * registro `{ only: [1, 3] }` con las páginas **1-based, ordenadas y sin
 * repetir**. No hay una tercera forma, y la gramática de lo que se teclea
 * (`1,2-3,10-20`) se traduce a esta antes de cruzar.
 */
export type PageSet = "all" | { only: number[] };

/**
 * El recuadro colocado: **dónde y en qué páginas** (ID-90).
 *
 * Sustituye al `SignaturePlacement { page, rect }` de v0.2. Es un registro
 * llano y no una unión de un brazo: un `kind` que nunca discrimina es ruido, y
 * el día que entre otra rama de colocación, `rect` y `pages` tendrán que
 * **desaparecer**, no convivir con ella.
 *
 * **«Colocado» no es una bandera: es tener al menos una página sellada**
 * (ID-92). Por eso no existe una colocación con el conjunto vacío: quitar la
 * última página devuelve `null`, que es exactamente el estado del PDF recién
 * abierto.
 */
export interface Placement {
  rect: UserSpaceRect;
  pages: PageSet;
}

/**
 * Cuál de las tres opciones del panel manda sobre el conjunto.
 *
 * El visor no la elige —vive en el panel— pero la necesita para dos cosas que
 * le pide el ID-101: la cuarta redacción del botón («Colocar el sello aquí»
 * cuando se sellan todas) y que con `Solo 1 página` o `Todas las páginas` una
 * página ya sellada **no ofrezca pastilla**, porque no queda nada que ofrecer.
 */
export type PageChoice = "single" | "these" | "all";

/** ¿Esta página lleva recuadro? Es la pregunta que contesta el ID-96. */
export function sealsPage(pages: PageSet, page: number): boolean {
  return pages === "all" || pages.only.includes(page);
}

/**
 * Las páginas que el conjunto nombra en un documento de `pageCount`.
 *
 * `"all"` y la lista completa dan lo mismo, igual que en `PageSet::resolve`.
 */
export function sealedPages(pages: PageSet, pageCount: number): number[] {
  if (pages !== "all") return pages.only;
  return Array.from({ length: Math.max(0, pageCount) }, (_, index) => index + 1);
}

/**
 * El conjunto ordenado y sin repetir, o `null` si no queda ninguna página.
 *
 * `null` no es un fallo: es el ID-92. Sin páginas no hay colocación, y quien lo
 * reciba tiene que **borrar el recuadro**, no guardar un conjunto vacío —que el
 * puente leería como «la última página»—.
 */
export function pageSetOf(pages: Iterable<number>): PageSet | null {
  const only = [...new Set(pages)].sort((a, b) => a - b);
  return only.length === 0 ? null : { only };
}

/** La primera página del conjunto: la que el visor abre y la que mide el panel. */
export function firstSealedPage(placement: Placement | null): number | null {
  if (placement === null) return null;
  if (placement.pages === "all") return 1;
  return placement.pages.only[0] ?? null;
}

/** Añade una página al conjunto. Con «todas» ya estaba dentro y no cambia nada. */
export function sealing(placement: Placement, page: number): Placement {
  if (placement.pages === "all") return placement;
  const pages = pageSetOf([...placement.pages.only, page]);
  return pages === null ? placement : { ...placement, pages };
}

/**
 * Quita una página del conjunto, y con la última **quita la colocación entera**
 * (ID-92).
 *
 * `"all"` se resuelve antes de restar, que es para lo que hace falta
 * `pageCount`: quitar una de «todas» deja a las demás nombradas una a una.
 */
export function unsealing(placement: Placement, page: number, pageCount: number): Placement | null {
  const rest = sealedPages(placement.pages, pageCount).filter((sealed) => sealed !== page);
  const pages = pageSetOf(rest);
  return pages === null ? null : { ...placement, pages };
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
 * La **posición estándar** del recuadro (ID-102).
 *
 * Colocado por la pastilla o por el campo de páginas no hay gesto que diga
 * dónde, así que cae abajo a la derecha a un **8 %** del borde, que es donde
 * suele ir la firma en un documento administrativo. Va en proporción a la
 * página para que el zoom no lo cambie de tamaño sobre el papel; a partir de
 * ahí se arrastra, que la posición es libre y no hay rejilla (ID-26).
 */
export function standardBox(page: PageSize): PixelRect {
  const width = page.width * 0.34;
  const height = page.height * 0.095;
  const margin = page.height * 0.08;
  return {
    x: page.width - width - page.width * 0.08,
    y: page.height - height - margin,
    width,
    height,
  };
}

/**
 * El **tamaño mínimo** del recuadro, en puntos de espacio de usuario (ID-103).
 *
 * Es «aquel por debajo del cual el nombre y la fecha ya no caben»: con las tres
 * líneas del sello administrativo —«Firmado por», el nombre con el DNI y la
 * fecha— a los 6 pt que es donde AutoFirma deja de bajar, la línea más larga
 * pide unos 120 pt de ancho y las tres, con su interlínea, unos 34 pt de alto.
 * Por debajo queda **la rúbrica sola**, y ahí es justo donde se paran los
 * tiradores: el gesto se detiene en vez de recortar el texto en silencio.
 *
 * Va en **puntos y no en píxeles** porque es una pregunta sobre el papel: lo
 * que cabe o no cabe dentro del sello no depende de cuánto se haya acercado
 * quien lo coloca.
 */
export const MIN_BOX_POINTS: PageSize = { width: 120, height: 34 };

/**
 * El lado del tirador, **en píxeles de pantalla** (ID-104).
 *
 * No escala con la hoja: mide lo mismo al 50 %, al 100 % y al 300 %, porque es
 * la diana del gesto y no parte del documento. El sello sí escala, porque es la
 * hoja.
 */
export const GRIP_PX = 10;

/** Por qué esquina se agarra el recuadro para redimensionarlo. */
export type BoxCorner = "top-left" | "top-right" | "bottom-left" | "bottom-right";

/**
 * El recuadro redimensionado por una esquina, en píxeles del lienzo.
 *
 * La esquina opuesta se queda quieta —es la que da el gesto su punto fijo— y
 * `min` para el tirador: por debajo del mínimo el rectángulo **no encoge más**,
 * y el resto del arrastre no hace nada. Con `keepRatio` (`Mayús`) se conserva
 * la proporción de partida, y el recorte al mínimo se aplica **después**, sobre
 * los dos lados a la vez, para que conservar la proporción no sea una puerta
 * trasera al tamaño ilegible.
 */
export function resizedBy(
  rect: PixelRect,
  corner: BoxCorner,
  dx: number,
  dy: number,
  min: PageSize,
  keepRatio: boolean,
): PixelRect {
  const towardsRight = corner === "top-right" || corner === "bottom-right";
  const towardsBottom = corner === "bottom-left" || corner === "bottom-right";
  let width = rect.width + (towardsRight ? dx : -dx);
  let height = rect.height + (towardsBottom ? dy : -dy);

  if (keepRatio && rect.height > 0) {
    // Manda el lado que más ha crecido en proporción: agarrando la esquina se
    // arrastra en diagonal y elegir siempre el mismo eje haría que el gesto
    // ignorase la mitad del recorrido.
    const ratio = rect.width / rect.height;
    width = Math.max(width, height * ratio);
    height = width / ratio;
    if (width < min.width) {
      width = min.width;
      height = width / ratio;
    }
    if (height < min.height) {
      height = min.height;
      width = height * ratio;
    }
  } else {
    width = Math.max(width, min.width);
    height = Math.max(height, min.height);
  }

  return {
    x: towardsRight ? rect.x : rect.x + rect.width - width,
    y: towardsBottom ? rect.y : rect.y + rect.height - height,
    width,
    height,
  };
}

/**
 * El conjunto que guarda **cada opción** del bloque «Colocación» (#188).
 *
 * Las tres opciones no se turnan sobre un mismo conjunto: cada una recuerda el
 * suyo, y elegir otra **no reescribe la que dejas**. Sin esto, sellar la 2 en
 * `Solo 1 página` se sumaba a la 1 en vez de sustituirla, y volver a `Estas
 * páginas` traía lo que hubiera dejado la opción anterior en vez del rango que
 * se tecleó allí.
 *
 * `all` no necesita hueco: su conjunto es la palabra `"all"` y no hay nada que
 * recordar. `single` guarda **un número** y no un `PageSet` porque una página
 * es lo único que esa opción puede llegar a nombrar; el tipo lo dice mejor que
 * una invariante escrita al lado.
 *
 * El **recuadro es uno solo** y no vive aquí: es el mismo rectángulo mirado
 * desde las tres opciones, y cambiar de opción no lo mueve.
 */
export interface PageSets {
  single: number | null;
  these: PageSet | null;
}

/** El documento recién abierto: ninguna opción ha nombrado todavía una página. */
export const NO_PAGE_SETS: PageSets = { single: null, these: null };

/** El conjunto de la opción activa, que es el único que manda sobre la firma. */
export function pagesOf(sets: PageSets, choice: PageChoice): PageSet | null {
  if (choice === "all") return "all";
  if (choice === "these") return sets.these;
  return sets.single === null ? null : { only: [sets.single] };
}

/**
 * La colocación que ve el resto de la ventana: el recuadro compartido y el
 * conjunto de la opción activa.
 *
 * Sigue valiendo el ID-92 —colocado es tener páginas—, solo que ahora «tener
 * páginas» se pregunta **por opción**: con el recuadro puesto y `Estas páginas`
 * sin rango, no hay colocación aunque `single` sí tenga la suya.
 */
export function placementOf(
  rect: UserSpaceRect | null,
  sets: PageSets,
  choice: PageChoice,
): Placement | null {
  if (rect === null) return null;
  const pages = pagesOf(sets, choice);
  return pages === null ? null : { rect, pages };
}

/** Guarda `pages` en la opción activa. Las otras dos **no se tocan**. */
export function storing(
  sets: PageSets,
  choice: PageChoice,
  pages: PageSet | null,
  pageCount: number,
): PageSets {
  // «Todas» no tiene conjunto que guardar: es la palabra, siempre la misma.
  if (choice === "all") return sets;
  if (choice === "these") return { ...sets, these: pages };
  return { ...sets, single: pages === null ? null : (sealedPages(pages, pageCount)[0] ?? null) };
}

/**
 * La opción que se activa, sembrada **solo si nunca tuvo conjunto propio**.
 *
 * Es la mitad que sigue debiéndose a la ficha: estrenar `Estas páginas` viniendo
 * de `Solo 1 página` = 3 arranca con `3` escrito. Lo que ya no ocurre es lo
 * contrario —volver a una opción que ya se usó trae **lo suyo**, no lo de la
 * anterior—, y por eso la siembra mira primero si hay algo guardado.
 *
 * `fallback` es la página que se está mirando: la única respuesta razonable
 * cuando `Solo 1 página` se estrena sin nada colocado en ninguna parte.
 */
export function activating(
  sets: PageSets,
  choice: PageChoice,
  previous: PageSet | null,
  pageCount: number,
  fallback: number,
): PageSets {
  if (choice === "all") return sets;
  if (choice === "these") {
    return sets.these === null ? { ...sets, these: previous } : sets;
  }
  if (sets.single !== null) return sets;
  const first = previous === null ? null : (sealedPages(previous, pageCount)[0] ?? null);
  return { ...sets, single: first ?? fallback };
}

/**
 * La **posición estándar en espacio de usuario**, que es la que puede pedir
 * quien no pinta nada (ID-102, #185).
 *
 * El issue #185 daba por hecho que colocar desde el panel exigía una costura
 * nueva con el visor «porque el viewport no sale de ahí». No es cierto: el
 * viewport a escala 1 lo da la propia página de `pdf.js`, y con él la posición
 * estándar sale de las dos funciones que ya existían, rotación incluida y sin
 * una segunda tabla por `/Rotate` —que es justo lo que la cabecera de este
 * módulo prohíbe—.
 */
export function standardRectOf(viewport: Viewport): UserSpaceRect {
  return toUserSpace(viewport, standardBox(viewport));
}
