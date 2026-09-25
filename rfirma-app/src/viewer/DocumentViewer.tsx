import { useEffect } from "react";
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
import type { StampPreview } from "../signing/stampPreview";
import "./DocumentViewer.css";
import type { PdfDocument } from "./pdf";
import { StampPill } from "./StampPill";
import { GRIP_PX, type PageChoice, type Placement } from "./signatureBox";
import type { DocumentFailure } from "./source";
import { useViewerBox } from "./useViewerBox";
import { useViewerPage } from "./useViewerPage";
import { ZOOM_MAX, ZOOM_MIN } from "./zoom";

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
   * El visor no la elige: la lee para saber si sellar sustituye o añade
   * (ID-96). Por omisión, `these`.
   */
  pageChoice?: PageChoice;
  /**
   * La página que se está mirando ha cambiado.
   *
   * El visor la sigue eligiendo él —el recorrido es suyo—, pero el panel la
   * necesita para elegir la cara del botón de sellar (#194, antes ID-100).
   */
  onPageChange?: (page: number) => void;
  /**
   * El botón de sellar vive en el panel, pero sellar y quitar el sello siguen
   * siendo del visor: es quien tiene el `viewport` de `pdf.js` que mide la
   * posición estándar del recuadro (#194).
   *
   * **Cada petición es un objeto nuevo**, igual que antes `goToPage`: pulsar
   * el mismo botón dos veces tiene que actuar las dos veces, así que lo que
   * dispara la acción es la identidad y no el valor.
   */
  placementRequest?: { action: "seal" | "unseal" } | null;
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
  onOpenHelp?: () => void;
  /**
   * Se puede colocar la firma visible ahora mismo: el interruptor está
   * encendido **y** hay un certificado utilizable (ID-108).
   *
   * Los tres caminos que colocan cuelgan de esto: sin ello no hay recuadro
   * pintado, no hay pastilla que ofrezca sellar y la hoja no traza. Es la misma
   * pregunta que el panel contesta apagando su bloque entero —«Elige un
   * certificado para colocar la firma visible»—, y el visor tenía su propia
   * copia del estado que no la respetaba (#190).
   *
   * La colocación **no se borra** al perderla: vive en quien la recuerda y
   * vuelve intacta, así que aquí sólo se deja de pintar.
   */
  canPlace?: boolean;
  /**
   * El documento **con el sello ya estampado**, que se pinta en lugar del
   * original mientras esté compuesto (ID-107).
   *
   * Es la pieza entera de «dentro del recuadro va el sello de verdad»: el visor
   * no dibuja nada nuevo, pinta otro PDF. Los bytes visibles de ese PDF están
   * medidos idénticos a los del firmado de verdad, así que el recuadro no
   * enseña una aproximación sino el resultado. `null` es no tener ninguno —sin
   * certificado, sin colocar, o no se ha podido componer—, y entonces se pinta
   * el original.
   */
  stamped?: PdfDocument | null;
  /**
   * El gesto está en curso: la vista anterior queda **congelada y atenuada**
   * (ID-109).
   *
   * Congelada sale gratis —quien no recompone es quien entrega `stamped`—; lo
   * que se pinta aquí es el atenuado, sobre el recuadro de antes del gesto.
   */
  stampFrozen?: boolean;
  /**
   * Empieza o acaba un gesto sobre el recuadro.
   *
   * Lo necesita quien compone el sello: recalcular por fotograma cuesta ≈1,9 s
   * y 507 MB de RSS en el peor documento medido, así que el ciclo se pide **al
   * soltar** y no durante el arrastre (ID-109).
   */
  onGesture?: (active: boolean) => void;
  /**
   * En qué estado está el sello que se ve **sobre la hoja** (ID-107, #202).
   *
   * Es lo único que la pastilla flotante cuenta: sin certificado, sin colocar
   * y «al día» no dicen nada —no hay sello del que hablar, o ya no hace falta
   * decirlo—, así que la pastilla no se monta para esos tres.
   */
  stamp?: StampPreview;
  /** «Ver cómo queda», y también «Volver a intentarlo». */
  onComposeStamp?: () => void;
}

/**
 * La columna central: **cómo va a quedar**.
 *
 * Es la parte imperativa de la interfaz. `pdf.js` pinta sobre un `<canvas>` y
 * devuelve tareas que siguen escribiendo después de lanzarlas, así que toda
 * la geometría que no sale del framework —la cola de pintadas, el zoom, los
 * tres gestos del recuadro— vive en [`useViewerPage`](./useViewerPage.ts) y
 * [`useViewerBox`](./useViewerBox.ts); este componente sólo es el JSX.
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
  onPageChange,
  placementRequest = null,
  canPlace = true,
  failure = null,
  onOpenHelp,
  stamped = null,
  stampFrozen = false,
  onGesture,
  stamp,
  onComposeStamp,
}: DocumentViewerProps) {
  const { t, i18n } = useTranslation();
  const {
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
  } = useViewerPage({ pdf, placement, stamped });

  useEffect(() => {
    onPageChange?.(page);
  }, [page, onPageChange]);

  const { boxElement, sheet, ghost, pixels, drag, tracing, gesturing, nudge } = useViewerBox({
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
  });

  const percent = new Intl.NumberFormat(i18n.language, {
    style: "percent",
    maximumFractionDigits: 0,
  });

  if (!pdf) {
    return (
      <div className="viewer viewer--empty">
        {failure && (
          <ErrorNotice
            situation={failure.situation}
            technicalDetail={failure.detail}
            onOpenHelp={onOpenHelp}
          />
        )}
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
          <ErrorNotice
            situation={failure.situation}
            technicalDetail={failure.detail}
            onOpenHelp={onOpenHelp}
          />
        </div>
      )}
      <div className="viewer__scroll" ref={setSurface}>
        <div
          ref={sheet}
          // El `crosshair` es lo único que anuncia el trazo: es el único de los
          // tres caminos que no tiene un botón ni un campo que lo cuente (#190).
          className={`viewer__sheet${canPlace ? " viewer__sheet--traceable" : ""}`}
          data-theme="light"
          role="document"
          aria-label={t("viewer.sheet")}
          // La hoja se enfoca para pasar de página con el teclado, y es también
          // donde acaban las teclas de página que burbujean desde el recuadro.
          // biome-ignore lint/a11y/noNoninteractiveTabindex: se enfoca y navega con el teclado.
          tabIndex={0}
          onKeyDown={navigate}
          style={{ width: viewport?.width, height: viewport?.height }}
          // Sin firma visible que colocar, la hoja no traza: no habría dónde
          // pintar lo que se colocara.
          {...(canPlace ? gesturing(tracing) : {})}
        >
          <canvas ref={canvas} className="viewer__canvas" />
          {/*
            El rectángulo del trazo en curso. Se pinta oculto y lo enseña el
            gesto escribiéndole el estilo: montarlo al empezar habría metido a
            React en un camino que existe justo para no pasar por él.
          */}
          <div
            ref={ghost}
            className="viewer__trace"
            aria-hidden="true"
            style={{ display: "none" }}
          />
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
              {...gesturing(drag.box)}
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
                  {...gesturing(drag.grip(corner))}
                />
              ))}
            </div>
          )}
          {/*
            El atenuado del gesto (ID-109). Va sobre el recuadro **de antes**,
            que es donde sigue el sello congelado: el que se arrastra se ha ido
            con el puntero, y atenuar la hoja entera atenuaría el documento, que
            es justo lo que hay que seguir viendo debajo.
          */}
          {stampFrozen && pixels && (
            <div
              className="viewer__stamp-frozen"
              aria-hidden="true"
              style={{
                left: `${pixels.x}px`,
                top: `${pixels.y}px`,
                width: `${pixels.width}px`,
                height: `${pixels.height}px`,
              }}
            />
          )}
        </div>
      </div>

      {/*
        Un solo inquilino en el hueco sobre la botonera (#202): `outOfPage` no
        se apaga al soltar, solo con la siguiente colocación válida (`place`,
        `seal`, `trace`, `unseal` o `goTo`), así que el aviso puede seguir
        puesto —y la pastilla del sello escondida con él— después de que el
        gesto termine, hasta ese próximo evento.
      */}
      {outOfPage ? (
        <p className="viewer__alert rf-body" role="alert">
          {t("viewer.outOfPage")}
        </p>
      ) : (
        stamp && <StampPill state={stamp} onCompose={onComposeStamp ?? noop} />
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
              event.preventDefault();
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

function noop() {}
