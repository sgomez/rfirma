import type { PageSize } from "./signatureBox";

/**
 * El zoom del visor: un rango continuo, «ajustar» como **modo**, y el tope del
 * mapa de bits.
 *
 * Aquí no hay React ni DOM: es aritmética, y por eso se prueba sola. El visor
 * pone el estado y los oyentes; este módulo contesta a «cuánto» (ID-116,
 * ID-117, ID-119).
 *
 * La distinción que sostiene el módulo es entre **cuánto quieres ampliar** y
 * **cómo quieres mirar**. Un porcentaje es lo primero y no sobrevive a nada:
 * cambiar de documento lo devuelve al 100 %. «Ajustar al ancho» es lo segundo,
 * y sobrevive al cambio de página, al redimensionado de la ventana y al
 * documento siguiente, porque no describe *ese* documento sino la forma de
 * mirarlos (ID-117).
 */

/** El zoom mínimo: por debajo, un A4 es una miniatura sobre la que no se coloca nada. */
export const ZOOM_MIN = 0.25;
/** El zoom máximo. Es lo que ve la persona; el mapa de bits tiene su propio tope. */
export const ZOOM_MAX = 4;

/**
 * Los escalones del zoom, los mismos siete de siempre (ID-116).
 *
 * Ya no son *el* zoom —eso es ahora el rango continuo— sino los **destinos con
 * los que tropiezan los botones ±**: el porcentaje se lee, se compara entre
 * sesiones y se dice en voz alta, así que pulsar «acercar» tiene que caer en un
 * número redondo y no en el 137 % en el que quedó el pellizco.
 */
export const ZOOM_STEPS = [0.5, 0.75, 1, 1.25, 1.5, 2, 3];

/**
 * El tope del mapa de bits: `zoom * devicePixelRatio` no pasa de 4× (ID-119).
 *
 * Un A4 al 400 % con `devicePixelRatio` 2 sería un lienzo de ~4 760 × 6 736 px,
 * **128 MB para una sola página**, y con el porcentaje editable ese techo se
 * alcanza tecleando. Lo que se recorta es la resolución del lienzo, no el zoom:
 * la persona sigue viendo el documento al 400 %. **No se avisa**, porque nadie
 * ha pedido un mapa de bits: han pedido verlo más grande, y lo ven.
 */
export const MAX_BITMAP_SCALE = 4;

/** El respiro que se le deja a la hoja al ajustar, para que no toque los bordes. */
const FIT_MARGIN = 0.92;

/** Redondeos: dos escalas que difieren en la billonésima son la misma. */
const EPSILON = 1e-6;

/**
 * Cómo se mira el documento.
 *
 * `libre` lleva el valor dentro porque es un dato del momento; los dos modos de
 * ajuste no llevan ninguno, y ese es justo el punto: **lo que se recuerda es el
 * modo, y la escala se recalcula** cada vez que cambia el tamaño de la
 * superficie o la página que se mira.
 */
export type ZoomMode =
  | { readonly kind: "free"; readonly value: number }
  | { readonly kind: "fit-width" }
  | { readonly kind: "fit-page" };

/** El punto de partida, y a donde vuelve `Ctrl+0` y el documento siguiente. */
export const DEFAULT_ZOOM: ZoomMode = { kind: "free", value: 1 };

/** Un zoom recortado al rango. Todo lo que sale de aquí ha pasado por esto. */
export function clampZoom(value: number): number {
  if (!Number.isFinite(value)) return 1;
  return Math.min(Math.max(value, ZOOM_MIN), ZOOM_MAX);
}

/**
 * El escalón siguiente en la dirección pedida, o el extremo del rango si ya no
 * quedan escalones por ese lado.
 *
 * Que el último escalón sea el 300 % y el techo el 400 % no es un descuido: los
 * botones recorren los números redondos y el rango completo se alcanza igual,
 * porque desde el 300 % un «acercar» más lleva al tope.
 */
export function steppedZoom(current: number, direction: 1 | -1): number {
  if (direction === 1) {
    return ZOOM_STEPS.find((step) => step > current + EPSILON) ?? ZOOM_MAX;
  }
  const below = ZOOM_STEPS.filter((step) => step < current - EPSILON);
  return below[below.length - 1] ?? ZOOM_MIN;
}

/**
 * El zoom tras un `Ctrl`+rueda —que es también como llega **el pellizco del
 * trackpad**, sin una línea de código aparte (ID-116)—.
 *
 * Es **multiplicativo**: la misma cantidad de rueda amplía lo mismo al 30 % que
 * al 300 %, que es lo que hace que el gesto se sienta igual en todo el rango.
 * Sumar sería lo contrario: imperceptible arriba y brusco abajo.
 */
export function pinchedZoom(current: number, deltaY: number): number {
  return clampZoom(current * Math.exp(-deltaY / 200));
}

/** Lo que ha desplazado la parte visible, en píxeles del lienzo. */
export interface ScrollOffset {
  left: number;
  top: number;
}

/**
 * El desplazamiento que deja **quieto bajo el puntero** el punto del documento
 * que había debajo (ID-116).
 *
 * `pointer` es la posición del puntero **relativa a la parte visible**, no a la
 * página. El punto del documento está a `scroll + pointer` del origen; tras
 * multiplicar la escala por `factor` está a `(scroll + pointer) * factor`, y
 * para que vuelva bajo el puntero hay que restarle otra vez `pointer`.
 *
 * Se aplica en el mismo cuadro que la escala: si se dejara para el efecto
 * siguiente, el documento daría un salto visible antes de recolocarse.
 */
export function anchoredScroll(
  scroll: ScrollOffset,
  pointer: { x: number; y: number },
  factor: number,
): ScrollOffset {
  return {
    left: Math.max(0, (scroll.left + pointer.x) * factor - pointer.x),
    top: Math.max(0, (scroll.top + pointer.y) * factor - pointer.y),
  };
}

/**
 * El porcentaje tecleado en la barra, ya recortado al rango, o `null` si lo
 * escrito no es un porcentaje.
 *
 * Se recorta en vez de rechazar: quien teclea 1000 quiere lo más grande que
 * haya, y devolverle el campo en rojo no le da nada. Se aceptan el signo y la
 * coma decimal porque es lo que sale de copiar el propio rótulo.
 */
export function typedZoom(text: string): number | null {
  const digits = text.replace(/[^\d.,]/g, "").replace(",", ".");
  if (digits === "") return null;
  const percent = Number.parseFloat(digits);
  if (!Number.isFinite(percent) || percent <= 0) return null;
  return clampZoom(percent / 100);
}

/**
 * La escala que pide un modo de ajuste, o `null` cuando no hay nada que
 * recalcular —zoom fijado a mano— o aún no hay medidas.
 *
 * `surface` es la parte visible del visor y `page` **la página sin escalar**,
 * en puntos de espacio de usuario: ajustar es una razón entre las dos, y
 * meterle el zoom actual la haría depender de sí misma.
 */
export function fitScale(
  mode: ZoomMode,
  surface: PageSize | null,
  page: PageSize | null,
): number | null {
  if (mode.kind === "free") return null;
  if (!surface || !page) return null;
  if (surface.width <= 0 || page.width <= 0 || page.height <= 0) return null;
  const byWidth = (surface.width * FIT_MARGIN) / page.width;
  if (mode.kind === "fit-width") return clampZoom(byWidth);
  if (surface.height <= 0) return null;
  return clampZoom(Math.min(byWidth, (surface.height * FIT_MARGIN) / page.height));
}

/**
 * A qué escala se pinta el mapa de bits, que **no** es la escala a la que se
 * mide nada: el viewport en píxeles CSS sigue siendo el del zoom, porque de él
 * salen las coordenadas del recuadro (ID-84).
 */
export function bitmapScale(zoom: number, devicePixelRatio: number): number {
  const ratio = Math.max(devicePixelRatio || 1, 1);
  return Math.min(zoom * ratio, MAX_BITMAP_SCALE);
}
