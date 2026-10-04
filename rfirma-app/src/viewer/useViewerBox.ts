//! Los gestos sobre el recuadro de la firma visible —arrastrar, redimensionar, trazar y moverlo con las flechas— y la petición de sellar o quitar el sello de la página que se mira; informa de ellos, no decide el conjunto de páginas.

import { type KeyboardEvent, useEffect, useRef } from "react";
import { type Placement, sealsPage, type UserSpaceRect } from "../placement/pageSets";
import type { Viewport } from "./pdf";
import {
  fitsInPage,
  MIN_BOX_POINTS,
  movedBy,
  type PixelRect,
  standardBox,
  toPixels,
  toUserSpace,
} from "./signatureBox";
import { type BoxDragHandlers, useBoxDrag } from "./useBoxDrag";
import { useBoxTrace } from "./useBoxTrace";

/**
 * Lo que se mueve el recuadro con una flecha, y con la flecha más `Shift`,
 * **en puntos de espacio de usuario**.
 *
 * En píxeles del lienzo el gesto dependía del zoom: al 300 % una flecha movía
 * un tercio de punto y al 50 % movía dos, así que colocar con precisión pedía
 * acercarse primero. Un punto es un punto a cualquier escala.
 */
const NUDGE = 1;
const NUDGE_FAST = 10;

interface UseViewerBoxArgs {
  placement: Placement | null;
  onMove?: (rect: UserSpaceRect) => void;
  onTrace?: (rect: UserSpaceRect, page: number) => void;
  onSeal?: (rect: UserSpaceRect, page: number) => void;
  onUnseal?: (page: number) => void;
  placementRequest: { action: "seal" | "unseal" } | null;
  canPlace: boolean;
  onGesture?: (active: boolean) => void;
  page: number;
  zoom: number;
  viewport: Viewport | null;
  surface: HTMLDivElement | null;
  setOutOfPage: (value: boolean) => void;
}

/**
 * Los tres caminos que colocan el recuadro de la firma visible —arrastrar,
 * redimensionar y trazar— y la pastilla de sellar/quitar sello que los
 * comparte. Hermano de [`useViewerPage`](./useViewerPage.ts), que es quien
 * pinta la hoja sobre la que este recuadro se dibuja.
 */
export function useViewerBox({
  placement,
  onMove,
  onTrace,
  onSeal,
  onUnseal,
  placementRequest,
  canPlace,
  onGesture,
  page,
  zoom,
  viewport,
  surface,
  setOutOfPage,
}: UseViewerBoxArgs) {
  const boxElement = useRef<HTMLDivElement>(null);
  const sheet = useRef<HTMLDivElement>(null);
  // El rectángulo que se dibuja mientras se traza. Está siempre en el DOM y
  // oculto: el trazo le escribe la geometría a mano, sin pasar por React.
  const ghost = useRef<HTMLDivElement>(null);
  // El recuadro acaba de nacer de un trazo y se lleva el foco en cuanto se
  // pinte: el gesto grueso y el ajuste fino con las flechas son un
  // solo movimiento.
  const focusBox = useRef(false);

  // El recuadro se pinta **idéntico en todas las páginas del conjunto y en
  // ninguna más**. La página donde se arrastró no se dibuja distinta
  // —inventaría una diferencia que el PDF no tiene—, y fuera del conjunto la
  // página va en blanco: ni un fantasma a trazos, que insinuaría que ahí hay
  // algo. Quien quiera saberlo lo lee en la pastilla, con palabras.
  const sealed = placement !== null && sealsPage(placement.pages, page);
  const pixels: PixelRect | null =
    viewport && placement && sealed && canPlace ? toPixels(viewport, placement.rect) : null;

  // Al cambiar de página, un recuadro que quede fuera de la parte visible se
  // trae a ella. **Una sola vez, en el cambio de página**: hacerlo al repintar
  // o al cambiar el zoom impediría mirar otra zona de la misma página.
  //
  // La página se da por atendida **en cuanto se ha podido mirar**, haya
  // recuadro o no: el recuadro se pinta sólo en su página, así que
  // marcarla sólo cuando lo hay dejaba la marca clavada en la página del
  // recuadro y el regreso a ella —el único caso que importa—
  // salía por la guarda sin hacer nada.
  const broughtIn = useRef(page);
  useEffect(() => {
    if (broughtIn.current === page) return;
    if (!surface || !viewport) return;
    broughtIn.current = page;
    const element = boxElement.current;
    if (!element) return;
    const box = element.getBoundingClientRect();
    const frame = surface.getBoundingClientRect();
    const seen =
      box.top >= frame.top &&
      box.bottom <= frame.bottom &&
      box.left >= frame.left &&
      box.right <= frame.right;
    if (!seen) element.scrollIntoView?.({ block: "nearest", inline: "nearest" });
  }, [page, viewport, surface]);

  /** Informa del recuadro movido o redimensionado, dado en píxeles del lienzo. */
  const place = (moved: PixelRect) => {
    if (!viewport || !placement) return;
    setOutOfPage(false);
    onMove?.(toUserSpace(viewport, moved));
  };

  // Los dos gestos avisan de que empiezan y de que acaban, sin que `useBoxDrag`
  // tenga que saber para qué: quien compone el sello sólo quiere el ciclo al
  // soltar, y el arrastre sigue sin pasar por el estado de React.
  const gesturing = (handlers: BoxDragHandlers): BoxDragHandlers => ({
    onPointerDown: (event) => {
      // Agarrar el recuadro **no es** trazar sobre la hoja: sin esto el mismo
      // `pointerdown` arrancaría los dos gestos, porque el recuadro vive dentro
      // de la hoja y el trazo escucha allí.
      event.stopPropagation();
      handlers.onPointerDown(event);
      // La misma guardia que `useBoxDrag`: con el botón secundario no arranca
      // ningún gesto, así que avisar de uno congelaría la vista previa hasta el
      // `pointerup` sin que se esté moviendo nada.
      if (event.button !== 0) return;
      onGesture?.(true);
    },
    onPointerMove: handlers.onPointerMove,
    onPointerUp: (event) => {
      handlers.onPointerUp(event);
      onGesture?.(false);
    },
    onPointerCancel: (event) => {
      handlers.onPointerCancel(event);
      onGesture?.(false);
    },
  });

  const drag = useBoxDrag({
    box: boxElement,
    rect: pixels ?? { x: 0, y: 0, width: 0, height: 0 },
    page: viewport ?? { width: 0, height: 0 },
    // El mínimo es del papel y la comparación de la pantalla: los puntos de
    // `MIN_BOX_POINTS`, a la escala a la que se está mirando.
    min: { width: MIN_BOX_POINTS.width * zoom, height: MIN_BOX_POINTS.height * zoom },
    onDrop: place,
    onOutOfPage: () => setOutOfPage(true),
  });

  /**
   * Sellar la página que se está mirando.
   *
   * Sin nada colocado, el recuadro nace en su **posición estándar** —no hay
   * gesto que diga dónde—; con algo colocado, el rectángulo no se mueve.
   */
  const seal = () => {
    if (!viewport) return;
    setOutOfPage(false);
    onSeal?.(placement?.rect ?? toUserSpace(viewport, standardBox(viewport)), page);
  };

  /** Informa del recuadro trazado sobre la hoja y de la página donde se trazó. */
  const trace = (traced: PixelRect) => {
    if (!viewport) return;
    setOutOfPage(false);
    focusBox.current = true;
    onTrace?.(toUserSpace(viewport, traced), page);
  };

  const tracing = useBoxTrace({
    sheet,
    ghost,
    page: viewport ?? { width: 0, height: 0 },
    min: { width: MIN_BOX_POINTS.width * zoom, height: MIN_BOX_POINTS.height * zoom },
    onTrace: trace,
  });

  // El foco al recuadro recién trazado, cuando ya está pintado. `pixels` es la
  // **señal** de que ya lo está —el recuadro no existe hasta que lo hay—, no
  // algo que este efecto lea: por eso el linter la ve de más.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `pixels` dispara el efecto, no lo alimenta.
  useEffect(() => {
    if (!focusBox.current || !boxElement.current) return;
    focusBox.current = false;
    boxElement.current.focus();
  }, [pixels]);

  const unseal = () => {
    if (placement === null) return;
    setOutOfPage(false);
    onUnseal?.(page);
  };

  // El botón que sella o quita el sello vive en el panel; la petición cruza
  // como `placementRequest` y se atiende aquí, que es donde vive el `viewport`. Se guarda la
  // identidad de la petición: pulsar el mismo botón dos veces tiene que actuar las dos veces,
  // aunque la acción no haya cambiado.
  const requestedPlacement = useRef(placementRequest);
  // biome-ignore lint/correctness/useExhaustiveDependencies: `seal` y `unseal` se recrean en cada pintada; lo que dispara el efecto es la identidad de `placementRequest`, no ellas.
  useEffect(() => {
    if (placementRequest === null || placementRequest === requestedPlacement.current) return;
    requestedPlacement.current = placementRequest;
    // Sin firma visible que colocar no hay nada que sellar: atenderla colocaba
    // un recuadro que después no se pintaba en ninguna parte.
    if (!canPlace) return;
    if (placementRequest.action === "seal") seal();
    else unseal();
  }, [placementRequest, canPlace]);

  /**
   * El recuadro atiende **sólo las flechas**, y `Esc` devuelve el foco a la
   * hoja. Todo lo demás —las teclas de página— se deja burbujear
   * hasta la hoja, así que se pasa de página sin salir del recuadro.
   *
   * El empuje es de **un punto de espacio de usuario**, y 10 con `Shift`: como
   * la guardia de «cabe en la página» trabaja en píxeles del lienzo, el paso se
   * convierte aquí, y a la escala del viewport un punto son `zoom` píxeles.
   */
  const nudge = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      sheet.current?.focus();
      return;
    }
    const step = (event.shiftKey ? NUDGE_FAST : NUDGE) * zoom;
    const towards: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    const direction = towards[event.key];
    if (!direction || !pixels || !viewport) return;
    event.preventDefault();
    const moved = movedBy(pixels, direction[0], direction[1]);
    if (fitsInPage(moved, viewport)) place(moved);
    else setOutOfPage(true);
  };

  return { boxElement, sheet, ghost, pixels, drag, tracing, gesturing, nudge };
}
