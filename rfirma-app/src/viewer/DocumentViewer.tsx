import { type KeyboardEvent, useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  ChevronsLeftIcon,
  ChevronsRightIcon,
  FitIcon,
  MinusIcon,
  MoveIcon,
  PlusIcon,
  UploadIcon,
} from "../design-system/icons";
import "./DocumentViewer.css";
import type { PdfDocument, Viewport } from "./pdf";
import { createRenderQueue, type RenderQueue } from "./renderQueue";
import {
  defaultBox,
  fitsInPage,
  movedBy,
  type PixelRect,
  type SignaturePlacement,
  toPixels,
  toUserSpace,
} from "./signatureBox";
import { useBoxDrag } from "./useBoxDrag";

/**
 * Los escalones del zoom. Son pasos y no una rueda continua porque el
 * porcentaje se lee, se compara entre sesiones y se dice en voz alta: «ponlo al
 * 150 %» no significa nada si cada acercamiento cae donde quiera.
 */
const ZOOM_STEPS = [0.5, 0.75, 1, 1.25, 1.5, 2, 3];

/** Lo que se mueve el recuadro con una flecha, y con la flecha más `Shift`. */
const NUDGE = 1;
const NUDGE_FAST = 10;

interface DocumentViewerProps {
  /** El documento abierto, o `null` si no hay ninguno. */
  pdf: PdfDocument | null;
  /** Dónde va la firma visible, en espacio de usuario. `null` mientras no hay. */
  placement: SignaturePlacement | null;
  /** El recuadro ha cambiado de sitio, de tamaño de página o de página. */
  onPlace: (placement: SignaturePlacement) => void;
  /** Abrir un documento, que va por el portal igual que desde la bandeja. */
  onOpen: () => void;
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
export function DocumentViewer({ pdf, placement, onPlace, onOpen }: DocumentViewerProps) {
  const { t, i18n } = useTranslation();
  const canvas = useRef<HTMLCanvasElement>(null);
  const boxElement = useRef<HTMLDivElement>(null);
  const surface = useRef<HTMLDivElement>(null);
  // Una cola por lienzo, creada una sola vez: es quien garantiza que no haya
  // dos pintadas vivas.
  const queue = useRef<RenderQueue | null>(null);
  queue.current ??= createRenderQueue();
  const [page, setPage] = useState(1);
  const [zoom, setZoom] = useState(1);
  const [viewport, setViewport] = useState<Viewport | null>(null);
  const [outOfPage, setOutOfPage] = useState(false);

  const pageCount = pdf?.pageCount ?? 0;

  // Documento nuevo, recorrido nuevo: primera página y escala original. Se
  // ajusta durante la pintada y no en un efecto porque es estado derivado de
  // una prop: con un efecto habría una pintada intermedia con la página del
  // documento anterior.
  const [shown, setShown] = useState(pdf);
  if (pdf !== shown) {
    setShown(pdf);
    setPage(1);
    setZoom(1);
    setViewport(null);
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
      const next = loaded.getViewport({ scale: zoom });
      target.width = next.width;
      target.height = next.height;
      setViewport(next);
      return pending.run(() => loaded.render({ canvas: target, viewport: next }));
    });

    return () => {
      live = false;
      pending.cancel();
    };
  }, [pdf, page, zoom]);

  // El recuadro tiene que existir y tiene que caber en la página que se está
  // mirando: la firma va donde estás mirando, y una página más pequeña que la
  // anterior no puede quedarse con un recuadro que se sale.
  useEffect(() => {
    if (!viewport) return;
    const kept = placement?.rect;
    const fits = kept !== undefined && fitsInPage(toPixels(viewport, kept), viewport);
    if (fits && placement?.page === page) return;
    onPlace({ page, rect: fits && kept ? kept : toUserSpace(viewport, defaultBox(viewport)) });
  }, [viewport, placement, page, onPlace]);

  const pixels: PixelRect | null =
    viewport && placement ? toPixels(viewport, placement.rect) : null;

  /** Confirma el recuadro movido, ya en píxeles del lienzo. */
  const place = useCallback(
    (moved: PixelRect) => {
      if (!viewport) return;
      setOutOfPage(false);
      onPlace({ page, rect: toUserSpace(viewport, moved) });
    },
    [viewport, page, onPlace],
  );

  const drag = useBoxDrag({
    box: boxElement,
    rect: pixels ?? { x: 0, y: 0, width: 0, height: 0 },
    page: viewport ?? { width: 0, height: 0 },
    onDrop: place,
    onOutOfPage: () => setOutOfPage(true),
  });

  /** Las flechas mueven el recuadro sin ratón, por la misma guardia. */
  const nudge = (event: KeyboardEvent<HTMLElement>) => {
    const step = event.shiftKey ? NUDGE_FAST : NUDGE;
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
    setPage(Math.min(Math.max(1, wanted), Math.max(1, pageCount)));
  };

  const stepZoom = (direction: 1 | -1) => {
    const index = ZOOM_STEPS.indexOf(zoom);
    const next = ZOOM_STEPS[(index === -1 ? nearestStep(zoom) : index) + direction];
    if (next !== undefined) setZoom(next);
  };

  /** Ajustar a la ventana: el ancho del visor manda, con un respiro a los lados. */
  const fitToWindow = () => {
    const available = surface.current?.clientWidth;
    if (!available || !viewport) return;
    setZoom((current) => (current * (available * 0.92)) / viewport.width);
  };

  const percent = new Intl.NumberFormat(i18n.language, {
    style: "percent",
    maximumFractionDigits: 0,
  });

  if (!pdf) {
    return (
      <div className="viewer viewer--empty">
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
    <div className="viewer" ref={surface}>
      <div className="viewer__scroll">
        <div
          className="viewer__sheet"
          data-theme="light"
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
              {...drag}
            >
              <span className="viewer__handle rf-body">
                <MoveIcon />
                {t("viewer.dragHandle")}
              </span>
            </div>
          )}
        </div>
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
        <input
          className="rf-input viewer__page"
          type="number"
          min={1}
          max={pageCount}
          aria-label={t("viewer.pageNumber")}
          value={page}
          onChange={(event) => goTo(Number(event.target.value))}
        />
        <span className="rf-body">{t("viewer.pageOf", { total: pageCount })}</span>
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
          disabled={zoom <= (ZOOM_STEPS[0] ?? 0)}
          onClick={() => stepZoom(-1)}
        >
          <MinusIcon />
        </button>
        <span className="rf-body viewer__zoom">{percent.format(zoom)}</span>
        <button
          type="button"
          className="rf-btn rf-btn--ghost viewer__step"
          aria-label={t("viewer.zoomIn")}
          disabled={zoom >= (ZOOM_STEPS[ZOOM_STEPS.length - 1] ?? 0)}
          onClick={() => stepZoom(1)}
        >
          <PlusIcon />
        </button>
        <button
          type="button"
          className="rf-btn rf-btn--ghost viewer__step"
          aria-label={t("viewer.fitToWindow")}
          onClick={fitToWindow}
        >
          <FitIcon />
        </button>
      </div>
    </div>
  );
}

/** El escalón más cercano a un zoom que no es ninguno, como el de «ajustar». */
function nearestStep(zoom: number): number {
  let best = 0;
  ZOOM_STEPS.forEach((step, index) => {
    const current = ZOOM_STEPS[best] ?? 1;
    if (Math.abs(step - zoom) < Math.abs(current - zoom)) best = index;
  });
  return best;
}
