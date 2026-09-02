import { type PointerEvent as ReactPointerEvent, type RefObject, useCallback, useRef } from "react";
import {
  type BoxCorner,
  fitsInPage,
  movedBy,
  type PageSize,
  type PixelRect,
  resizedBy,
} from "./signatureBox";

interface BoxDragOptions {
  /** El elemento del recuadro. Es a él a quien se le escribe el gesto en curso. */
  box: RefObject<HTMLElement | null>;
  /** Dónde está pintado ahora, en píxeles del lienzo. */
  rect: PixelRect;
  /** El lienzo, para la guardia del ID-22. */
  page: PageSize;
  /**
   * El tamaño por debajo del cual los tiradores no bajan, **en píxeles del
   * lienzo** (ID-103).
   *
   * Llega convertido y no como los puntos de `MIN_BOX_POINTS` porque el gesto
   * entero trabaja en píxeles: el mínimo es del papel, pero la comparación es
   * de la pantalla.
   */
  min: PageSize;
  /** El recuadro soltado, ya comprobado que cabe. */
  onDrop: (moved: PixelRect) => void;
  /** Se ha soltado fuera de la página y no se acepta. */
  onOutOfPage: () => void;
}

/** Los manejadores que se derraman sobre un elemento del gesto. */
interface BoxDragHandlers {
  onPointerDown: (event: ReactPointerEvent<HTMLElement>) => void;
  onPointerMove: (event: ReactPointerEvent<HTMLElement>) => void;
  onPointerUp: (event: ReactPointerEvent<HTMLElement>) => void;
  onPointerCancel: (event: ReactPointerEvent<HTMLElement>) => void;
}

/** Los dos gestos del recuadro, cada uno para su elemento. */
interface BoxDrag {
  /** Mover: van sobre el recuadro entero. */
  box: BoxDragHandlers;
  /** Redimensionar: van sobre el tirador de esa esquina. */
  grip: (corner: BoxCorner) => BoxDragHandlers;
}

/** El gesto en curso. Vive en una `ref`, nunca en el estado. */
interface Gesture {
  pointerId: number;
  /** Por qué esquina se agarró, o `null` si se agarró el recuadro para moverlo. */
  corner: BoxCorner | null;
  /** Dónde se agarró, en coordenadas de la ventana. */
  x: number;
  y: number;
  /** Dónde estaba el recuadro al agarrarlo. */
  from: PixelRect;
  /** Lo que lleva desplazado, que es lo que se confirma al soltar. */
  dx: number;
  dy: number;
  /** Si se pidió conservar la proporción (`Mayús`) en el último movimiento. */
  keepRatio: boolean;
}

/**
 * Arrastrar y redimensionar el recuadro **sin pasar por el estado de React**.
 *
 * Durante el gesto lo único que cambia es la geometría en línea del elemento,
 * escrita a mano sobre la `ref`. El estado se toca **una vez**, al soltar.
 * Pasar cada `pointermove` por `setState` significa reconciliar el árbol entero
 * sesenta veces por segundo con un `<canvas>` de un PDF al lado, y se nota.
 *
 * El puntero se **captura** al agarrar, así que el gesto sigue vivo aunque el
 * cursor se salga del recuadro —que es justo lo que pasa al arrastrar deprisa—.
 *
 * Los tiradores de las esquinas (ID-103) usan el mismo gesto con otra
 * aritmética: la esquina opuesta se queda quieta, `Mayús` conserva la
 * proporción y por debajo del tamaño mínimo **el gesto se detiene** en vez de
 * recortar el texto del sello en silencio.
 *
 * Soltar fuera de la página no se acepta: se avisa y el recuadro vuelve donde
 * estaba (ID-22). Recortar en silencio es lo que hace iText, y es el fallo del
 * que este proyecto se defiende.
 */
export function useBoxDrag({ box, rect, page, min, onDrop, onOutOfPage }: BoxDragOptions): BoxDrag {
  const gesture = useRef<Gesture | null>(null);

  /**
   * Deshace lo que el gesto escribió a mano: a partir de aquí pinta el estado.
   *
   * El movimiento va por `transform` y el redimensionado por la geometría en
   * línea, y esta segunda **no puede limpiarse a secas**: la posición y el
   * tamaño los escribe React en el mismo `style`, así que se reponen los de la
   * prop —que son los que el estado tiene ahora mismo—.
   */
  const restore = useCallback(() => {
    const element = box.current;
    if (!element) return;
    element.style.transform = "";
    element.style.left = `${rect.x}px`;
    element.style.top = `${rect.y}px`;
    element.style.width = `${rect.width}px`;
    element.style.height = `${rect.height}px`;
  }, [box, rect]);

  /** Lo que el gesto ha construido hasta ahora, sin confirmar. */
  const shapeOf = (current: Gesture): PixelRect =>
    current.corner === null
      ? movedBy(current.from, current.dx, current.dy)
      : resizedBy(current.from, current.corner, current.dx, current.dy, min, current.keepRatio);

  const start = (corner: BoxCorner | null) => (event: ReactPointerEvent<HTMLElement>) => {
    // Solo el botón principal: con el secundario se abre el menú del sistema.
    if (event.button !== 0) return;
    // Agarrar un tirador no es agarrar el recuadro: sin esto el mismo gesto
    // arrancaría los dos y el recuadro se movería mientras se redimensiona.
    if (corner !== null) event.stopPropagation();
    event.currentTarget.setPointerCapture?.(event.pointerId);
    gesture.current = {
      pointerId: event.pointerId,
      corner,
      x: event.clientX,
      y: event.clientY,
      from: rect,
      dx: 0,
      dy: 0,
      keepRatio: event.shiftKey,
    };
  };

  const move = (event: ReactPointerEvent<HTMLElement>) => {
    const current = gesture.current;
    if (!current || current.pointerId !== event.pointerId || !box.current) return;
    current.dx = event.clientX - current.x;
    current.dy = event.clientY - current.y;
    // `Mayús` se lee en cada movimiento: se pulsa y se suelta a media faena.
    current.keepRatio = event.shiftKey;
    if (current.corner === null) {
      box.current.style.transform = `translate(${current.dx}px, ${current.dy}px)`;
      return;
    }
    const shape = shapeOf(current);
    box.current.style.left = `${shape.x}px`;
    box.current.style.top = `${shape.y}px`;
    box.current.style.width = `${shape.width}px`;
    box.current.style.height = `${shape.height}px`;
  };

  const end = (event: ReactPointerEvent<HTMLElement>) => {
    const current = gesture.current;
    if (!current || current.pointerId !== event.pointerId) return;
    event.currentTarget.releasePointerCapture?.(event.pointerId);
    gesture.current = null;
    restore();

    // La forma sale del último `pointermove`, no del `pointerup`: soltar sin
    // haber movido no trae coordenadas útiles.
    const shape = shapeOf(current);
    if (fitsInPage(shape, page)) onDrop(shape);
    else onOutOfPage();
  };

  const cancel = (event: ReactPointerEvent<HTMLElement>) => {
    if (gesture.current?.pointerId !== event.pointerId) return;
    gesture.current = null;
    restore();
  };

  const handlersFor = (corner: BoxCorner | null): BoxDragHandlers => ({
    onPointerDown: start(corner),
    onPointerMove: move,
    onPointerUp: end,
    onPointerCancel: cancel,
  });

  return { box: handlersFor(null), grip: (corner) => handlersFor(corner) };
}
