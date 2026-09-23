import {
  type KeyboardEvent,
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type { PdfDocument, Viewport } from "./pdf";
import { createRenderQueue, type ObservedSize, observeSize, type RenderQueue } from "./renderQueue";
import { firstSealedPage, type PageSize, type Placement } from "./signatureBox";
import {
  anchoredScroll,
  bitmapScale,
  DEFAULT_ZOOM,
  fitScale,
  pinchedZoom,
  type ScrollOffset,
  steppedZoom,
  typedZoom,
  type ZoomMode,
} from "./zoom";

/** Las teclas de página. A cuál lleva cada una lo reparte `navigate`. */
const PAGE_KEYS = new Set(["PageDown", "PageUp", "Home", "End"]);

interface UseViewerPageArgs {
  pdf: PdfDocument | null;
  placement: Placement | null;
  stamped: PdfDocument | null;
}

/**
 * La pintada sobre el `<canvas>`, el recorrido de páginas y el zoom —
 * continuo, «ajustar» como modo y anclado al puntero—. Hermano de
 * [`useViewerBox`](./useViewerBox.ts), que es quien coloca el recuadro sobre
 * lo que esto pinta.
 */
export function useViewerPage({ pdf, placement, stamped }: UseViewerPageArgs) {
  const canvas = useRef<HTMLCanvasElement>(null);
  // La parte visible: la que se mide para ajustar, la que se desplaza al
  // anclar el zoom al puntero y la que dice si el recuadro se ve o no.
  //
  // Va en estado y no en un `ref` **a propósito**: sólo existe en la rama con
  // documento, y el visor se monta con `pdf === null`. Con un `ref`, todo
  // efecto que quisiera engancharse a ella corría en el montaje, la encontraba
  // vacía y no volvía a correr jamás; con estado, el elemento avisa de que ya
  // está ahí y los efectos se rehacen solos.
  const [surface, setSurface] = useState<HTMLDivElement | null>(null);
  // Una cola por lienzo, creada una sola vez: es quien garantiza que no haya
  // dos pintadas vivas.
  const queue = useRef<RenderQueue | null>(null);
  queue.current ??= createRenderQueue();
  // La página inicial sale del `placement`: montado ya con un documento
  // delante, no hay cambio de `pdf` que reponga la página que guardaba su fila.
  const [page, setPage] = useState(() =>
    within(firstSealedPage(placement) ?? 1, pdf?.pageCount ?? 0),
  );
  const [zoom, setZoom] = useState(1);
  // Cómo se mira, que no es lo mismo que cuánto se amplía: sobrevive al cambio
  // de página, al redimensionado y al documento siguiente, sea un modo de
  // ajuste o un porcentaje fijado a mano (ID-117 enmendado).
  const [mode, setMode] = useState<ZoomMode>(DEFAULT_ZOOM);
  const [viewport, setViewport] = useState<Viewport | null>(null);
  // La página **sin escalar**, en puntos: el divisor de todo ajuste. Sale de la
  // misma pintada, así que no hay una segunda lectura del documento.
  const [pagePoints, setPagePoints] = useState<PageSize | null>(null);
  const [visible, setVisible] = useState<ObservedSize | null>(null);
  const [outOfPage, setOutOfPage] = useState(false);
  // Lo tecleado en el porcentaje mientras se teclea. `null` = no se está
  // tecleando, y entonces el campo muestra el zoom de verdad.
  const [typing, setTyping] = useState<string | null>(null);
  // El desplazamiento que deja quieto el punto bajo el puntero, a la espera de
  // que la hoja crezca: aplicado antes, el navegador lo recortaría al tamaño
  // viejo.
  const anchor = useRef<ScrollOffset | null>(null);

  const pageCount = pdf?.pageCount ?? 0;

  // Lo que se pinta es el PDF **en seco** cuando lo hay, y el original cuando
  // no. El recorrido —la página, el zoom, el conjunto de páginas— sigue
  // colgando de `pdf`: los dos documentos tienen las mismas páginas, y hacer
  // que el visor se reiniciara cada vez que llega un sello nuevo sería devolver
  // a quien está colocando el recuadro a la página 1.
  const painted = stamped ?? pdf;

  // Documento nuevo, recorrido nuevo: **la página que guardaba su fila** —la 1
  // si no guardaba ninguna— y el zoom de partida.
  //
  // Se ajusta durante la pintada y no en un efecto porque es estado derivado de
  // una prop: con un efecto habría una pintada intermedia con la página del
  // documento anterior.
  const [shown, setShown] = useState(pdf);
  if (pdf !== shown) {
    setShown(pdf);
    setPage(within(firstSealedPage(placement) ?? 1, pdf?.pageCount ?? 0));
    setViewport(null);
    setPagePoints(null);
    // El modo cruza al documento siguiente entero, sea de ajuste o un
    // porcentaje fijado a mano: manda lo último que haya dicho la persona
    // usuaria, y el ajuste de partida —`DEFAULT_ZOOM`— solo se ve en el primer
    // documento, mientras no lo haya tocado (ID-117 enmendado).
  }

  // La pintada. Es el único sitio que toca el lienzo, y su limpieza cancela lo
  // que hubiera en vuelo: sin eso, al cambiar el zoom deprisa dos `RenderTask`
  // escriben sobre el mismo `<canvas>` y queda una mezcla de dos escalas.
  useEffect(() => {
    const pending = queue.current;
    if (!painted || !pending) return;
    const target = canvas.current;
    if (!target) return;
    let live = true;

    void painted.getPage(page).then((loaded) => {
      if (!live) return undefined;
      // `next` es el viewport en **píxeles CSS**: es el que se guarda en el
      // estado y el que usan las conversiones a espacio de usuario PDF
      // (`toPixels`/`toUserSpace`, y detrás de ellas `signing::placement`), así
      // que la nitidez de abajo no puede tocarlo o el `/Rect` que acaba en el
      // PDF cambiaría con la pantalla en la que se firmó (ID-84).
      const next = loaded.getViewport({ scale: zoom });
      // El mapa de bits se pinta a `devicePixelRatio`, para que el documento se
      // vea nítido en pantallas HiDPI; el tamaño en CSS —lo que ocupa en la
      // ventana— se fija aparte y no cambia, porque si no el navegador lo
      // reescalaría igual que a 1x y la nitidez no se notaría. Y se acota a 4×,
      // o el 400 % en una pantalla HiDPI serían 128 MB de lienzo (ID-119).
      const scale = bitmapScale(zoom, window.devicePixelRatio);
      const bitmap = scale === zoom ? next : loaded.getViewport({ scale });
      target.width = bitmap.width;
      target.height = bitmap.height;
      target.style.width = `${next.width}px`;
      target.style.height = `${next.height}px`;
      // La página sin escalar, que es contra lo que se ajusta. Sale del mismo
      // objeto ya cargado: `next.width / zoom` sería lo mismo salvo por el
      // redondeo, y ajustar por un tamaño redondeado se nota.
      const unscaled = loaded.getViewport({ scale: 1 });
      // El mismo tamaño se guarda como el mismo objeto: si cada pintada dejara
      // uno nuevo, el efecto de ajuste se dispararía en cada repintado y
      // reaplicaría una escala vieja sobre el zoom que se acaba de fijar a
      // mano.
      setPagePoints((current) =>
        current?.width === unscaled.width && current.height === unscaled.height
          ? current
          : { width: unscaled.width, height: unscaled.height },
      );
      setViewport(next);
      return pending.run(() => loaded.render({ canvas: target, viewport: bitmap }));
    });

    return () => {
      live = false;
      pending.cancel();
    };
  }, [painted, page, zoom]);

  // La parte visible se mide sola y avisa de cada cambio, que es lo que hace de
  // «ajustar» un modo y no un cálculo de una vez (ID-117).
  useEffect(() => {
    if (!surface) return;
    setVisible({ width: surface.clientWidth, height: surface.clientHeight });
    return observeSize(surface, setVisible);
  }, [surface]);

  // Ajustar es una razón entre la parte visible y la página: cambie la que
  // cambie, la escala se recalcula. Con el zoom fijado a mano no hay nada que
  // recalcular y `fitScale` devuelve `null`.
  useEffect(() => {
    const wanted = fitScale(mode, visible, pagePoints);
    if (wanted !== null) {
      setZoom((current) => (Math.abs(current - wanted) < 1e-6 ? current : wanted));
    }
  }, [mode, visible, pagePoints]);

  // El desplazamiento anclado al puntero se aplica cuando la hoja ya ha crecido
  // —el mismo cuadro en el que cambia la escala—, nunca antes.
  //
  // biome-ignore lint/correctness/useExhaustiveDependencies: `viewport` no se lee, dispara: es la señal de que la hoja ya tiene el tamaño nuevo.
  useLayoutEffect(() => {
    const wanted = anchor.current;
    if (!surface || !wanted) return;
    anchor.current = null;
    surface.scrollLeft = wanted.left;
    surface.scrollTop = wanted.top;
  }, [viewport]);

  const goTo = (wanted: number) => {
    setOutOfPage(false);
    setPage(within(wanted, pageCount));
  };

  /** Un zoom fijado a mano, que es lo que saca del modo de ajuste (ID-117). */
  const toZoom = useCallback((value: number) => {
    setMode({ kind: "free", value });
    setZoom(value);
  }, []);

  /**
   * La hoja atiende las teclas de página, y también las del recuadro cuando
   * burbujean desde él (ID-113). `Ctrl+0` vuelve al 100 % (ID-116).
   */
  const navigate = (event: KeyboardEvent<HTMLElement>) => {
    if (event.ctrlKey && event.key === "0") {
      event.preventDefault();
      toZoom(1);
      return;
    }
    if (!PAGE_KEYS.has(event.key)) return;
    event.preventDefault();
    if (event.key === "PageDown") goTo(page + 1);
    else if (event.key === "PageUp") goTo(page - 1);
    else if (event.key === "Home") goTo(1);
    else goTo(pageCount);
  };

  /**
   * `Ctrl`+rueda amplía **anclado al puntero**, y el pellizco del trackpad
   * llega por aquí sin una línea aparte: el navegador lo entrega como una rueda
   * con `ctrlKey` (ID-116).
   */
  const zoomAtPointer = useCallback(
    (event: globalThis.WheelEvent) => {
      if (!event.ctrlKey) return;
      event.preventDefault();
      const next = pinchedZoom(zoom, event.deltaY);
      if (next === zoom) return;
      if (surface) {
        const frame = surface.getBoundingClientRect();
        // Un pellizco de trackpad emite decenas de eventos seguidos, y el
        // desplazamiento del anterior aún no se ha aplicado —lo aplica el
        // `useLayoutEffect` cuando llega el viewport nuevo—. Partir del
        // `scroll` del elemento anclaría sobre un valor viejo, así que
        // mientras haya uno pendiente se compone sobre él.
        const from = anchor.current ?? { left: surface.scrollLeft, top: surface.scrollTop };
        anchor.current = anchoredScroll(
          from,
          { x: event.clientX - frame.left, y: event.clientY - frame.top },
          next / zoom,
        );
      }
      toZoom(next);
    },
    [zoom, surface, toZoom],
  );

  // React marca `wheel` como oyente **pasivo** (`addTrappedEventListener`, en
  // `DOMPluginEventSystem`), y dentro de uno pasivo `preventDefault()` no hace
  // nada: con la prop `onWheel`, `Ctrl`+rueda y el pellizco conservaban su
  // acción por defecto y ampliaban el WebView entero además del documento. El
  // oyente se engancha a mano, con `passive: false`.
  useEffect(() => {
    if (!surface) return;
    surface.addEventListener("wheel", zoomAtPointer, { passive: false });
    return () => surface.removeEventListener("wheel", zoomAtPointer);
  }, [surface, zoomAtPointer]);

  /** Los botones ± tropiezan con los siete escalones, no con el continuo. */
  const stepZoom = (direction: 1 | -1) => toZoom(steppedZoom(zoom, direction));

  /** Toma lo tecleado en el porcentaje, recortado al rango, y suelta el campo. */
  const commitTyped = () => {
    const wanted = typing === null ? null : typedZoom(typing);
    setTyping(null);
    if (wanted !== null) toZoom(wanted);
  };

  return {
    canvas,
    surface,
    setSurface,
    page,
    pageCount,
    zoom,
    mode,
    setMode,
    viewport,
    outOfPage,
    setOutOfPage,
    typing,
    setTyping,
    goTo,
    navigate,
    stepZoom,
    commitTyped,
  };
}

/** La página `wanted` recortada a las que tiene el documento. */
function within(wanted: number, pageCount: number): number {
  return Math.min(Math.max(1, wanted), Math.max(1, pageCount));
}
