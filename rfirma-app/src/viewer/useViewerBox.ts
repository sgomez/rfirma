import { type KeyboardEvent, useEffect, useRef } from "react";
import type { Viewport } from "./pdf";
import {
  fitsInPage,
  MIN_BOX_POINTS,
  movedBy,
  type PageChoice,
  type PixelRect,
  type Placement,
  sealing,
  sealsPage,
  standardBox,
  toPixels,
  toUserSpace,
  type UserSpaceRect,
  unsealing,
} from "./signatureBox";
import { type BoxDragHandlers, useBoxDrag } from "./useBoxDrag";
import { useBoxTrace } from "./useBoxTrace";

/**
 * Lo que se mueve el recuadro con una flecha, y con la flecha más `Shift`,
 * **en puntos de espacio de usuario** (ID-115).
 *
 * En píxeles del lienzo el gesto dependía del zoom: al 300 % una flecha movía
 * un tercio de punto y al 50 % movía dos, así que colocar con precisión pedía
 * acercarse primero. Un punto es un punto a cualquier escala.
 */
const NUDGE = 1;
const NUDGE_FAST = 10;

interface UseViewerBoxArgs {
  placement: Placement | null;
  onPlace: (placement: Placement | null) => void;
  pageChoice: PageChoice;
  placementRequest: { action: "seal" | "unseal" } | null;
  canPlace: boolean;
  onGesture?: (active: boolean) => void;
  page: number;
  pageCount: number;
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
  onPlace,
  pageChoice,
  placementRequest,
  canPlace,
  onGesture,
  page,
  pageCount,
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
  // pinte: el gesto grueso y el ajuste fino con las flechas (ID-115) son un
  // solo movimiento.
  const focusBox = useRef(false);

  // El recuadro se pinta **idéntico en todas las páginas del conjunto y en
  // ninguna más** (ID-96). La página donde se arrastró no se dibuja distinta
  // —inventaría una diferencia que el PDF no tiene—, y fuera del conjunto la
  // página va en blanco: ni un fantasma a trazos, que insinuaría que ahí hay
  // algo. Quien quiera saberlo lo lee en la pastilla, con palabras.
  const sealed = placement !== null && sealsPage(placement.pages, page);
  const pixels: PixelRect | null =
    viewport && placement && sealed && canPlace ? toPixels(viewport, placement.rect) : null;

  // Al cambiar de página, un recuadro que quede fuera de la parte visible se
  // trae a ella. **Una sola vez, en el cambio de página**: hacerlo al repintar
  // o al cambiar el zoom impediría mirar otra zona de la misma página (ID-118).
  //
  // La página se da por atendida **en cuanto se ha podido mirar**, haya
  // recuadro o no: el recuadro se pinta sólo en su página (ID-96), así que
  // marcarla sólo cuando lo hay dejaba la marca clavada en la página del
  // recuadro y el regreso a ella —el único caso que el ID-118 quiere cubrir—
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

  /**
   * Confirma el recuadro movido o redimensionado, ya en píxeles del lienzo.
   *
   * El conjunto de páginas **no lo toca el gesto**: mover el recuadro de una
   * página del conjunto lo mueve en todas, porque es un solo campo de firma con
   * el widget replicado (ID-96).
   */
  const place = (moved: PixelRect) => {
    if (!viewport || !placement) return;
    setOutOfPage(false);
    onPlace({ ...placement, rect: toUserSpace(viewport, moved) });
  };

  // Los dos gestos avisan de que empiezan y de que acaban, sin que `useBoxDrag`
  // tenga que saber para qué: quien compone el sello sólo quiere el ciclo al
  // soltar, y el arrastre sigue sin pasar por el estado de React.
  const gesturing = (handlers: BoxDragHandlers): BoxDragHandlers => ({
    onPointerDown: (event) => {
      // Agarrar el recuadro **no es** trazar sobre la hoja: sin esto el mismo
      // `pointerdown` arrancaría los dos gestos, porque el recuadro vive dentro
      // de la hoja y el trazo escucha allí (#190).
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
    // El mínimo es del papel y la comparación de la pantalla: los puntos del
    // ID-103, a la escala a la que se está mirando.
    min: { width: MIN_BOX_POINTS.width * zoom, height: MIN_BOX_POINTS.height * zoom },
    onDrop: place,
    onOutOfPage: () => setOutOfPage(true),
  });

  /**
   * La colocación que resulta de sellar la página que se mira, con el recuadro
   * en `rect` (ID-101, ID-102).
   *
   * Es la regla del conjunto y **la comparten los dos gestos que sellan**: la
   * pastilla, que no toca el rectángulo, y el trazo, que trae uno nuevo (#190).
   * Trazar es «sellar esta página» con sitio elegido, así que dos reglas
   * habrían sido dos maneras de contestar a la misma pregunta.
   *
   * Con `Solo 1 página` sellar **sustituye**: esa opción no puede nombrar dos, y
   * sumar aquí dejaba la 1 y la 2 selladas a la vez con el panel diciendo
   * «Página 1» (#188). Con las otras dos se añade, que es lo que significan.
   */
  const placedAt = (rect: UserSpaceRect): Placement => {
    if (placement === null) {
      return { rect, pages: pageChoice === "all" ? "all" : { only: [page] } };
    }
    if (pageChoice === "single") return { rect, pages: { only: [page] } };
    return { ...sealing(placement, page), rect };
  };

  /**
   * Sellar la página que se está mirando.
   *
   * Sin nada colocado, el recuadro nace en su **posición estándar** —no hay
   * gesto que diga dónde—; con algo colocado, el rectángulo no se mueve.
   */
  const seal = () => {
    if (!viewport) return;
    setOutOfPage(false);
    onPlace(placedAt(placement?.rect ?? toUserSpace(viewport, standardBox(viewport))));
  };

  /**
   * El recuadro trazado sobre la hoja, que es el gesto que lo hace nacer (#190).
   *
   * Trazar dice dos cosas —esta página y aquí— y se aplican las dos: el
   * rectángulo se mueve **en todas las páginas del conjunto**, porque el PDF
   * lleva un solo campo de firma con el widget replicado (ID-96), y el conjunto
   * cambia según la opción activa, igual que al sellar.
   */
  const trace = (traced: PixelRect) => {
    if (!viewport) return;
    setOutOfPage(false);
    focusBox.current = true;
    onPlace(placedAt(toUserSpace(viewport, traced)));
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

  /** Quitar el sello de esta página, y con el último, la colocación entera. */
  const unseal = () => {
    if (placement === null) return;
    setOutOfPage(false);
    onPlace(unsealing(placement, page, pageCount));
  };

  // El botón que sella o quita el sello vive en el panel; la petición cruza
  // como `placementRequest` y se atiende aquí, que es donde vive el `viewport`
  // (#194). El guardado por identidad es el mismo patrón que usaba
  // `goToPage`: pulsar el mismo botón dos veces tiene que actuar las dos
  // veces, aunque la acción no haya cambiado.
  const requestedPlacement = useRef(placementRequest);
  // biome-ignore lint/correctness/useExhaustiveDependencies: `seal` y `unseal` se recrean en cada pintada; lo que dispara el efecto es la identidad de `placementRequest`, no ellas.
  useEffect(() => {
    if (placementRequest === null || placementRequest === requestedPlacement.current) return;
    requestedPlacement.current = placementRequest;
    // Sin firma visible que colocar no hay nada que sellar: atenderla colocaba
    // un recuadro que después no se pintaba en ninguna parte (#190).
    if (!canPlace) return;
    if (placementRequest.action === "seal") seal();
    else unseal();
  }, [placementRequest, canPlace]);

  /**
   * El recuadro atiende **sólo las flechas**, y `Esc` devuelve el foco a la
   * hoja (ID-113). Todo lo demás —las teclas de página— se deja burbujear
   * hasta la hoja, así que se pasa de página sin salir del recuadro.
   *
   * El empuje es de **un punto de espacio de usuario**, y 10 con `Shift`: como
   * la guardia de «cabe en la página» trabaja en píxeles del lienzo, el paso se
   * convierte aquí, y a la escala del viewport un punto son `zoom` píxeles
   * (ID-115).
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
