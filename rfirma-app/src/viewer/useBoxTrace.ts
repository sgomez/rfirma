import { type PointerEvent as ReactPointerEvent, type RefObject, useRef } from "react";
import { type PageSize, type PixelPoint, type PixelRect, tracedBox } from "./signatureBox";
import type { BoxDragHandlers } from "./useBoxDrag";

/**
 * El desplazamiento por debajo del cual no hay trazo, **en píxeles de pantalla**.
 *
 * No escala con el zoom, por la misma razón que el lado de los tiradores
 * (ID-104): mide la intención de la mano, no el documento. Y existe porque la
 * hoja es también lo que se enfoca para pasar de página con el teclado, así que
 * un clic seco sobre ella significa «dame el foco» y no puede pasar a colocar
 * una firma.
 */
export const TRACE_THRESHOLD_PX = 4;

interface BoxTraceOptions {
  /** La hoja: de su rectángulo salen las coordenadas del lienzo. */
  sheet: RefObject<HTMLElement | null>;
  /**
   * El rectángulo que se dibuja mientras dura el trazo.
   *
   * Está siempre en el DOM y oculto: como en `useBoxDrag`, lo que cambia
   * durante el gesto es su geometría en línea, escrita a mano, y no el estado.
   */
  ghost: RefObject<HTMLElement | null>;
  /** El lienzo, que es lo que el trazo no puede desbordar. */
  page: PageSize;
  /** El mínimo del ID-103, ya en píxeles del lienzo. */
  min: PageSize;
  /** Se ha trazado un recuadro: esto es la colocación nueva. */
  onTrace: (traced: PixelRect) => void;
}

/** El trazo en curso. Vive en una `ref`, nunca en el estado. */
interface Trace {
  pointerId: number;
  /** Dónde arrancó, en píxeles del lienzo. */
  from: PixelPoint;
  /** Dónde va, en píxeles del lienzo, o `null` si aún no ha pasado del umbral. */
  to: PixelPoint | null;
}

/**
 * **Trazar** el recuadro sobre la hoja: el gesto que lo hace nacer (#190).
 *
 * Es el hermano de [`useBoxDrag`](./useBoxDrag.ts) y no un tercer modo suyo, y
 * la diferencia está en el ciclo de vida: aquel edita un elemento que ya está
 * en la página —le escribe el `transform` al recuadro de verdad—, y éste dibuja
 * algo que todavía no existe. Comparten la aritmética, que es donde está la
 * regla ([`tracedBox`](./signatureBox.ts)), y ahí acaba el parecido.
 *
 * De los tres caminos que colocan la firma —trazar, la pastilla y el campo de
 * páginas— es el único que elige **sitio** en el mismo gesto. Los otros dos
 * ponen el recuadro en su posición estándar, porque no hay nada que diga dónde.
 *
 * El gesto que nace dentro del recuadro no llega hasta aquí: ése es de
 * `useBoxDrag` y lo detiene él.
 */
export function useBoxTrace({
  sheet,
  ghost,
  page,
  min,
  onTrace,
}: BoxTraceOptions): BoxDragHandlers {
  const trace = useRef<Trace | null>(null);

  /** Del puntero al lienzo. La hoja es el origen de coordenadas del recuadro. */
  const pointOf = (event: ReactPointerEvent<HTMLElement>): PixelPoint | null => {
    const frame = sheet.current?.getBoundingClientRect();
    if (!frame) return null;
    return { x: event.clientX - frame.left, y: event.clientY - frame.top };
  };

  /** Enseña u oculta el fantasma, que es lo único que se ve durante el trazo. */
  const paint = (shape: PixelRect | null) => {
    const element = ghost.current;
    if (!element) return;
    if (shape === null) {
      element.style.display = "none";
      return;
    }
    element.style.display = "block";
    element.style.left = `${shape.x}px`;
    element.style.top = `${shape.y}px`;
    element.style.width = `${shape.width}px`;
    element.style.height = `${shape.height}px`;
  };

  const start = (event: ReactPointerEvent<HTMLElement>) => {
    // Solo el botón principal: con el secundario se abre el menú del sistema.
    if (event.button !== 0) return;
    const from = pointOf(event);
    if (!from) return;
    event.currentTarget.setPointerCapture?.(event.pointerId);
    trace.current = { pointerId: event.pointerId, from, to: null };
  };

  const move = (event: ReactPointerEvent<HTMLElement>) => {
    const current = trace.current;
    if (!current || current.pointerId !== event.pointerId) return;
    const to = pointOf(event);
    if (!to) return;
    // Hasta pasar el umbral no hay trazo **ni fantasma**: mientras el gesto
    // pueda todavía ser un clic, no se dibuja nada.
    if (
      current.to === null &&
      Math.abs(to.x - current.from.x) < TRACE_THRESHOLD_PX &&
      Math.abs(to.y - current.from.y) < TRACE_THRESHOLD_PX
    ) {
      return;
    }
    current.to = to;
    paint(tracedBox(current.from, to, page, min));
  };

  const end = (event: ReactPointerEvent<HTMLElement>) => {
    const current = trace.current;
    if (!current || current.pointerId !== event.pointerId) return;
    event.currentTarget.releasePointerCapture?.(event.pointerId);
    trace.current = null;
    paint(null);
    // La forma sale del último `pointermove`: sin ninguno que pasara el umbral,
    // esto fue un clic y la hoja se queda con el foco y nada más.
    if (current.to === null) return;
    onTrace(tracedBox(current.from, current.to, page, min));
  };

  const cancel = (event: ReactPointerEvent<HTMLElement>) => {
    if (trace.current?.pointerId !== event.pointerId) return;
    trace.current = null;
    paint(null);
  };

  return { onPointerDown: start, onPointerMove: move, onPointerUp: end, onPointerCancel: cancel };
}
