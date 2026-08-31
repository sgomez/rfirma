import { type PointerEvent as ReactPointerEvent, type RefObject, useCallback, useRef } from "react";
import { fitsInPage, movedBy, type PageSize, type PixelRect } from "./signatureBox";

interface BoxDragOptions {
  /** El elemento del recuadro. Es a él a quien se le escribe el `transform`. */
  box: RefObject<HTMLElement | null>;
  /** Dónde está pintado ahora, en píxeles del lienzo. */
  rect: PixelRect;
  /** El lienzo, para la guardia del ID-22. */
  page: PageSize;
  /** El recuadro soltado, ya comprobado que cabe. */
  onDrop: (moved: PixelRect) => void;
  /** Se ha soltado fuera de la página y no se acepta. */
  onOutOfPage: () => void;
}

/** Los manejadores que se derraman sobre el elemento del recuadro. */
interface BoxDragHandlers {
  onPointerDown: (event: ReactPointerEvent<HTMLElement>) => void;
  onPointerMove: (event: ReactPointerEvent<HTMLElement>) => void;
  onPointerUp: (event: ReactPointerEvent<HTMLElement>) => void;
  onPointerCancel: (event: ReactPointerEvent<HTMLElement>) => void;
}

/** El gesto en curso. Vive en una `ref`, nunca en el estado. */
interface Gesture {
  pointerId: number;
  /** Dónde se agarró, en coordenadas de la ventana. */
  x: number;
  y: number;
  /** Dónde estaba el recuadro al agarrarlo. */
  from: PixelRect;
  /** Lo que lleva desplazado, que es lo que se confirma al soltar. */
  dx: number;
  dy: number;
}

/**
 * Arrastrar el recuadro **sin pasar por el estado de React**.
 *
 * Durante el gesto lo único que cambia es el `transform` del elemento, escrito
 * a mano sobre la `ref`. El estado se toca **una vez**, al soltar. Pasar cada
 * `pointermove` por `setState` significa reconciliar el árbol entero sesenta
 * veces por segundo con un `<canvas>` de un PDF al lado, y se nota.
 *
 * El puntero se **captura** al agarrar, así que el gesto sigue vivo aunque el
 * cursor se salga del recuadro —que es justo lo que pasa al arrastrar deprisa—.
 *
 * Soltar fuera de la página no se acepta: se avisa y el recuadro vuelve donde
 * estaba (ID-22). Recortar en silencio es lo que hace iText, y es el fallo del
 * que este proyecto se defiende.
 */
export function useBoxDrag({
  box,
  rect,
  page,
  onDrop,
  onOutOfPage,
}: BoxDragOptions): BoxDragHandlers {
  const gesture = useRef<Gesture | null>(null);

  /** Quita el `transform` del gesto: a partir de aquí pinta el estado. */
  const clearTransform = useCallback(() => {
    if (box.current) box.current.style.transform = "";
  }, [box]);

  const onPointerDown = useCallback(
    (event: ReactPointerEvent<HTMLElement>) => {
      // Solo el botón principal: con el secundario se abre el menú del sistema.
      if (event.button !== 0) return;
      event.currentTarget.setPointerCapture?.(event.pointerId);
      gesture.current = {
        pointerId: event.pointerId,
        x: event.clientX,
        y: event.clientY,
        from: rect,
        dx: 0,
        dy: 0,
      };
    },
    [rect],
  );

  const onPointerMove = useCallback(
    (event: ReactPointerEvent<HTMLElement>) => {
      const current = gesture.current;
      if (!current || current.pointerId !== event.pointerId || !box.current) return;
      current.dx = event.clientX - current.x;
      current.dy = event.clientY - current.y;
      box.current.style.transform = `translate(${current.dx}px, ${current.dy}px)`;
    },
    [box],
  );

  const onPointerUp = useCallback(
    (event: ReactPointerEvent<HTMLElement>) => {
      const current = gesture.current;
      if (!current || current.pointerId !== event.pointerId) return;
      event.currentTarget.releasePointerCapture?.(event.pointerId);
      gesture.current = null;
      clearTransform();

      // El desplazamiento sale del último `pointermove`, no del `pointerup`:
      // soltar sin haber movido no trae coordenadas útiles.
      const moved = movedBy(current.from, current.dx, current.dy);
      if (fitsInPage(moved, page)) onDrop(moved);
      else onOutOfPage();
    },
    [clearTransform, onDrop, onOutOfPage, page],
  );

  const onPointerCancel = useCallback(
    (event: ReactPointerEvent<HTMLElement>) => {
      if (gesture.current?.pointerId !== event.pointerId) return;
      gesture.current = null;
      clearTransform();
    },
    [clearTransform],
  );

  return { onPointerDown, onPointerMove, onPointerUp, onPointerCancel };
}
