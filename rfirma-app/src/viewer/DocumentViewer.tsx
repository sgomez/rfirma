import {
  type KeyboardEvent,
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  ChevronsLeftIcon,
  ChevronsRightIcon,
  FitIcon,
  FitPageIcon,
  MinusIcon,
  MoveIcon,
  PlusIcon,
  UploadIcon,
} from "../design-system/icons";
import { ErrorNotice } from "../errors/ErrorNotice";
import "./DocumentViewer.css";
import type { PdfDocument, Viewport } from "./pdf";
import { createRenderQueue, type ObservedSize, observeSize, type RenderQueue } from "./renderQueue";
import {
  firstSealedPage,
  fitsInPage,
  GRIP_PX,
  MIN_BOX_POINTS,
  movedBy,
  type PageChoice,
  type PageSize,
  type PixelRect,
  type Placement,
  sealing,
  sealsPage,
  standardBox,
  toPixels,
  toUserSpace,
  unsealing,
} from "./signatureBox";
import type { DocumentFailure } from "./source";
import { useBoxDrag } from "./useBoxDrag";
import {
  anchoredScroll,
  bitmapScale,
  DEFAULT_ZOOM,
  fitScale,
  pinchedZoom,
  type ScrollOffset,
  steppedZoom,
  typedZoom,
  ZOOM_MAX,
  ZOOM_MIN,
  type ZoomMode,
} from "./zoom";

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

/** Las teclas de página. A cuál lleva cada una lo reparte `navigate`. */
const PAGE_KEYS = new Set(["PageDown", "PageUp", "Home", "End"]);

/** Las cuatro esquinas, con la clase que las coloca. */
const GRIPS = [
  { corner: "top-left", modifier: "tl" },
  { corner: "top-right", modifier: "tr" },
  { corner: "bottom-left", modifier: "bl" },
  { corner: "bottom-right", modifier: "br" },
] as const;

interface DocumentViewerProps {
  /** El documento abierto, o `null` si no hay ninguno. */
  pdf: PdfDocument | null;
  /**
   * Dónde va la firma visible y en qué páginas, en espacio de usuario. `null`
   * es **el documento recién abierto**: sin ninguna página sellada no hay
   * recuadro en ninguna parte (ID-92).
   */
  placement: Placement | null;
  /**
   * El recuadro ha cambiado de sitio, de tamaño o de conjunto de páginas.
   *
   * `null` es quitar la colocación entera, que es lo que deja quitar el sello
   * de la última página del conjunto (ID-92).
   */
  onPlace: (placement: Placement | null) => void;
  /**
   * Cuál de las tres opciones del panel manda sobre el conjunto de páginas.
   *
   * El visor no la elige: la lee para redactar la pastilla (ID-101). Por
   * omisión, `these`, que es la única de las tres en la que la pastilla ofrece
   * las tres caras.
   */
  pageChoice?: PageChoice;
  /** Abrir un documento, que va por el portal igual que desde la bandeja. */
  onOpen: () => void;
  /**
   * Por qué no se ha podido pintar el documento que se eligió, si es que no se
   * ha podido.
   *
   * Va aquí y no al pie del panel de firma porque sin documento abierto no hay
   * panel: quien acaba de elegir un PDF corrupto tiene el visor delante, y
   * dejarlo en su estado vacío contaba lo mismo que no haber abierto nada.
   *
   * Y se pinta **también con un documento delante**: el segundo PDF que no se
   * deja abrir deja el primero en pantalla, así que sin esto el rechazo era
   * mudo y parecía que la pulsación no había hecho nada.
   */
  failure?: DocumentFailure | null;
}

/**
 * La columna central: **cómo va a quedar**.
 *
 * Es la parte imperativa de la interfaz. `pdf.js` pinta sobre un `<canvas>` y
 * devuelve tareas que siguen escribiendo después de lanzarlas, así que dos
 * cosas no salen del framework y están escritas a mano:
 *
 * - las pintadas pasan por una [cola](./renderQueue.ts) que **cancela** la
 *   anterior al cambiar el zoom o la página;
 * - el arrastre del recuadro no toca el estado de React hasta soltar
 *   ([`useBoxDrag`](./useBoxDrag.ts)).
 *
 * El recuadro vive **en espacio de usuario PDF** (ID-21): los píxeles se
 * derivan del viewport en cada pintada, nunca al revés, así que el zoom es
 * puramente visual. Y se coloca **libremente**, sin rejilla (ID-26).
 *
 * **No hay pan por arrastre.** El documento se desplaza con la barra de
 * desplazamiento y la rueda, que es lo que ya hace el WebView, así que el
 * arrastre del ratón es siempre del recuadro y los dos gestos no compiten
 * —tampoco con el zoom al 300 %, donde el recuadro ocupa casi todo el visor—.
 * La discusión está en `docs/design/visor-de-documento.md`.
 */
export function DocumentViewer({
  pdf,
  placement,
  onPlace,
  onOpen,
  pageChoice = "these",
  failure = null,
}: DocumentViewerProps) {
  const { t, i18n } = useTranslation();
  const canvas = useRef<HTMLCanvasElement>(null);
  const boxElement = useRef<HTMLDivElement>(null);
  const sheet = useRef<HTMLDivElement>(null);
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
  // Cómo se mira, que no es lo mismo que cuánto se amplía: un modo de ajuste
  // sobrevive al cambio de página, al redimensionado y al documento siguiente;
  // un porcentaje fijado a mano, no (ID-117).
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
    // El modo de ajuste **sí** cruza al documento siguiente y el porcentaje
    // no: quien dijo «al ancho» describió cómo quiere mirar, no cuánto quiere
    // ampliar *este* documento (ID-117).
    if (mode.kind === "free") {
      setMode(DEFAULT_ZOOM);
      setZoom(1);
    }
  }

  // La pintada. Es el único sitio que toca el lienzo, y su limpieza cancela lo
  // que hubiera en vuelo: sin eso, al cambiar el zoom deprisa dos `RenderTask`
  // escriben sobre el mismo `<canvas>` y queda una mezcla de dos escalas.
  useEffect(() => {
    const pending = queue.current;
    if (!pdf || !pending) return;
    const target = canvas.current;
    if (!target) return;
    let live = true;

    void pdf.getPage(page).then((loaded) => {
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
  }, [pdf, page, zoom]);

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

  // El recuadro se pinta **idéntico en todas las páginas del conjunto y en
  // ninguna más** (ID-96). La página donde se arrastró no se dibuja distinta
  // —inventaría una diferencia que el PDF no tiene—, y fuera del conjunto la
  // página va en blanco: ni un fantasma a trazos, que insinuaría que ahí hay
  // algo. Quien quiera saberlo lo lee en la pastilla, con palabras.
  const sealed = placement !== null && sealsPage(placement.pages, page);
  const pixels: PixelRect | null =
    viewport && placement && sealed ? toPixels(viewport, placement.rect) : null;

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
  const place = useCallback(
    (moved: PixelRect) => {
      if (!viewport || !placement) return;
      setOutOfPage(false);
      onPlace({ ...placement, rect: toUserSpace(viewport, moved) });
    },
    [viewport, placement, onPlace],
  );

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
   * Sellar la página que se está mirando (ID-101, ID-102).
   *
   * Sin nada colocado, el recuadro nace en su **posición estándar** —no hay
   * gesto que diga dónde—; con algo colocado, la página se añade al conjunto y
   * el rectángulo no se mueve.
   */
  const seal = () => {
    if (!viewport) return;
    setOutOfPage(false);
    if (placement !== null) {
      onPlace(sealing(placement, page));
      return;
    }
    onPlace({
      rect: toUserSpace(viewport, standardBox(viewport)),
      pages: pageChoice === "all" ? "all" : { only: [page] },
    });
  };

  /** Quitar el sello de esta página, y con el último, la colocación entera. */
  const unseal = () => {
    if (placement === null) return;
    setOutOfPage(false);
    onPlace(unsealing(placement, page, pageCount));
  };

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

  const percent = new Intl.NumberFormat(i18n.language, {
    style: "percent",
    maximumFractionDigits: 0,
  });

  /**
   * La pastilla bajo la hoja, y cuál de sus tres caras toca (ID-101).
   *
   * `null` es **no hay pastilla**: con `Solo 1 página` o `Todas las páginas` y
   * la página ya sellada no queda nada que ofrecer ahí. Y el botón cambia de
   * texto con la opción: con «todas» el conjunto ya está completo y lo único
   * que falta es el rectángulo, así que decir «esta página» prometería una
   * página cuando se sellan las veintisiete.
   */
  const pill = ((): { text: string; label: string; variant: string; act: () => void } | null => {
    if (placement === null) {
      return {
        text: t("viewer.pill.notPlaced"),
        label: pageChoice === "all" ? t("viewer.pill.placeHere") : t("viewer.pill.seal"),
        variant: "rf-btn--primary",
        act: seal,
      };
    }
    if (!sealed) {
      return {
        text: t("viewer.pill.notSealed"),
        label: t("viewer.pill.seal"),
        variant: "rf-btn--secondary",
        act: seal,
      };
    }
    if (pageChoice !== "these") return null;
    return {
      text: t("viewer.pill.sealed"),
      label: t("viewer.pill.unseal"),
      variant: "rf-btn--ghost",
      act: unseal,
    };
  })();

  /** Toma lo tecleado en el porcentaje, recortado al rango, y suelta el campo. */
  const commitTyped = () => {
    const wanted = typing === null ? null : typedZoom(typing);
    setTyping(null);
    if (wanted !== null) toZoom(wanted);
  };

  if (!pdf) {
    return (
      <div className="viewer viewer--empty">
        {failure && <ErrorNotice situation={failure.situation} technicalDetail={failure.detail} />}
        <button type="button" className="viewer__drop-zone" onClick={onOpen}>
          <span className="viewer__drop-icon">
            <UploadIcon />
          </span>
          <span className="rf-title viewer__drop-title">{t("viewer.dropZone")}</span>
          <span className="rf-prose rf-text-muted">{t("viewer.dropZoneHint")}</span>
        </button>
        <p className="rf-prose rf-text-muted">{t("viewer.privacy")}</p>
      </div>
    );
  }

  return (
    <div className="viewer">
      {failure && (
        // Flota sobre la hoja, como la barra y el aviso de «se sale de la
        // página»: `.viewer` es una rejilla de una sola fila y meter aquí un
        // hijo en flujo le robaría altura al documento.
        <div className="viewer__failure">
          <ErrorNotice situation={failure.situation} technicalDetail={failure.detail} />
        </div>
      )}
      <div className="viewer__scroll" ref={setSurface}>
        <div
          ref={sheet}
          className="viewer__sheet"
          data-theme="light"
          role="document"
          aria-label={t("viewer.sheet")}
          // La hoja se enfoca para pasar de página con el teclado, y es también
          // donde acaban las teclas de página que burbujean desde el recuadro.
          // biome-ignore lint/a11y/noNoninteractiveTabindex: se enfoca y navega con el teclado.
          tabIndex={0}
          onKeyDown={navigate}
          style={{ width: viewport?.width, height: viewport?.height }}
        >
          <canvas ref={canvas} className="viewer__canvas" />
          {pixels && (
            <div
              ref={boxElement}
              className="viewer__box"
              role="application"
              aria-label={t("viewer.signatureBox")}
              // El recuadro se enfoca y se empuja con las flechas: que el linter
              // no vea interactivo un `div` no lo convierte en decoración.
              // biome-ignore lint/a11y/noNoninteractiveTabindex: se enfoca y se mueve con el teclado.
              tabIndex={0}
              style={{
                left: `${pixels.x}px`,
                top: `${pixels.y}px`,
                width: `${pixels.width}px`,
                height: `${pixels.height}px`,
              }}
              onKeyDown={nudge}
              {...drag.box}
            >
              <span className="viewer__handle rf-body">
                <MoveIcon />
                {t("viewer.dragHandle")}
              </span>
              {/*
                Los tiradores son **cromo, no papel** (ID-104): el lado va en
                línea, en píxeles de pantalla, para que mida lo mismo al 50 %,
                al 100 % y al 300 %. El recuadro sí escala, porque es la hoja.
              */}
              {GRIPS.map(({ corner, modifier }) => (
                <span
                  key={corner}
                  className={`viewer__grip viewer__grip--${modifier}`}
                  data-corner={corner}
                  style={{ width: `${GRIP_PX}px`, height: `${GRIP_PX}px` }}
                  aria-hidden="true"
                  {...drag.grip(corner)}
                />
              ))}
            </div>
          )}
        </div>

        {pill !== null && (
          // Bajo la hoja y centrada, que es literalmente donde va: es el único
          // camino para elegir páginas que no pasa por teclear (ID-101).
          <div className="viewer__pill rf-row">
            <span className="rf-body">{pill.text}</span>
            <button type="button" className={`rf-btn ${pill.variant}`} onClick={pill.act}>
              {pill.label}
            </button>
          </div>
        )}
      </div>

      {outOfPage && (
        <p className="viewer__alert rf-body" role="alert">
          {t("viewer.outOfPage")}
        </p>
      )}

      <div className="viewer__bar rf-row">
        <button
          type="button"
          className="rf-btn rf-btn--ghost viewer__step"
          aria-label={t("viewer.firstPage")}
          disabled={page === 1}
          onClick={() => goTo(1)}
        >
          <ChevronsLeftIcon />
        </button>
        <button
          type="button"
          className="rf-btn rf-btn--ghost viewer__step"
          aria-label={t("viewer.previousPage")}
          disabled={page === 1}
          onClick={() => goTo(page - 1)}
        >
          <ChevronLeftIcon />
        </button>
        <div className="rf-row rf-gap-xs viewer__pages">
          <input
            className="rf-input viewer__page"
            type="number"
            min={1}
            max={pageCount}
            aria-label={t("viewer.pageNumber")}
            value={page}
            onChange={(event) => goTo(Number(event.target.value))}
          />
          <span className="rf-body rf-text-muted">{t("viewer.pageOf", { total: pageCount })}</span>
        </div>
        <button
          type="button"
          className="rf-btn rf-btn--ghost viewer__step"
          aria-label={t("viewer.nextPage")}
          disabled={page === pageCount}
          onClick={() => goTo(page + 1)}
        >
          <ChevronRightIcon />
        </button>
        <button
          type="button"
          className="rf-btn rf-btn--ghost viewer__step"
          aria-label={t("viewer.lastPage")}
          disabled={page === pageCount}
          onClick={() => goTo(pageCount)}
        >
          <ChevronsRightIcon />
        </button>

        <span className="viewer__divider rf-divider" />

        <button
          type="button"
          className="rf-btn rf-btn--ghost viewer__step"
          aria-label={t("viewer.zoomOut")}
          disabled={zoom <= ZOOM_MIN}
          onClick={() => stepZoom(-1)}
        >
          <MinusIcon />
        </button>
        {/*
          El porcentaje se teclea: con el zoom continuo, los botones ya no
          alcanzan cualquier valor, y «ponlo al 150 %» tiene que poder escribirse
          (ID-116). Se recorta al rango en vez de rechazarse.
        */}
        <input
          className="rf-input viewer__zoom"
          type="text"
          inputMode="numeric"
          aria-label={t("viewer.zoomLevel")}
          value={typing ?? percent.format(zoom)}
          onChange={(event) => setTyping(event.target.value)}
          onFocus={(event) => event.target.select()}
          onBlur={commitTyped}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              commitTyped();
            } else if (event.key === "Escape") {
              setTyping(null);
            }
          }}
        />
        <button
          type="button"
          className="rf-btn rf-btn--ghost viewer__step"
          aria-label={t("viewer.zoomIn")}
          disabled={zoom >= ZOOM_MAX}
          onClick={() => stepZoom(1)}
        >
          <PlusIcon />
        </button>
        <button
          type="button"
          className="rf-btn rf-btn--ghost viewer__step"
          aria-label={t("viewer.fitWidth")}
          aria-pressed={mode.kind === "fit-width"}
          onClick={() => setMode({ kind: "fit-width" })}
        >
          <FitIcon />
        </button>
        <button
          type="button"
          className="rf-btn rf-btn--ghost viewer__step"
          aria-label={t("viewer.fitPage")}
          aria-pressed={mode.kind === "fit-page"}
          onClick={() => setMode({ kind: "fit-page" })}
        >
          <FitPageIcon />
        </button>
      </div>
    </div>
  );
}

/** La página `wanted` recortada a las que tiene el documento. */
function within(wanted: number, pageCount: number): number {
  return Math.min(Math.max(1, wanted), Math.max(1, pageCount));
}
