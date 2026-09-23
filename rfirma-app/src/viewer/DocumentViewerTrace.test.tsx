import { fireEvent, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { DocumentViewer } from "./DocumentViewer";
import type { PdfDocument } from "./pdf";
import { movedBy, type PageChoice, type Placement, toPixels, toUserSpace } from "./signatureBox";
import {
  box,
  goToPage,
  noop,
  recordingDocument,
  seated,
  sheet,
  viewportAt,
} from "./testing/documentViewerFixtures";

/**
 * **Grada A** (`vitest`, carril rápido). Sub-issue #58.
 *
 * El trazo del recuadro sobre la hoja, el único de los tres gestos que elige
 * sitio. El resto, en `DocumentViewerBox.test.tsx`.
 */

describe("el recuadro trazado sobre la hoja", () => {
  /** El visor con la colocación en estado, que es como lo monta `App`. */
  function Placing({
    document,
    pageChoice = "these",
    start = null,
    onPlace,
  }: {
    document: PdfDocument;
    pageChoice?: PageChoice;
    start?: Placement | null;
    onPlace?: (next: Placement | null) => void;
  }) {
    const [placement, setPlacement] = useState<Placement | null>(start);
    return (
      <DocumentViewer
        pdf={document}
        placement={placement}
        pageChoice={pageChoice}
        onPlace={(next) => {
          setPlacement(next);
          onPlace?.(next);
        }}
        onOpen={noop}
      />
    );
  }

  /** Pulsar, mover y soltar sobre la hoja, que es el gesto entero. */
  function traceOver(element: HTMLElement, from: [number, number], to: [number, number]) {
    fireEvent.pointerDown(element, { pointerId: 1, button: 0, clientX: from[0], clientY: from[1] });
    fireEvent.pointerMove(element, { pointerId: 1, clientX: to[0], clientY: to[1] });
    fireEvent.pointerUp(element, { pointerId: 1, clientX: to[0], clientY: to[1] });
  }

  it("draws a box where the pointer was dragged across an empty sheet", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(<Placing document={document} onPlace={onPlace} />);

    await waitFor(() => expect(renders).toHaveLength(1));
    expect(screen.queryByRole("application")).not.toBeInTheDocument();

    traceOver(sheet(), [100, 100], [300, 200]);

    expect(box()).toBeInTheDocument();
    // La hoja está en el origen (jsdom no mide), así que los píxeles del trazo
    // son los del puntero, y de ahí salen los puntos del ID-21.
    expect(onPlace).toHaveBeenCalledWith({
      rect: toUserSpace(viewportAt(1), { x: 100, y: 100, width: 200, height: 100 }),
      pages: { only: [1] },
    });
  });

  // El umbral es de la mano, no del papel: sin él, un clic en la hoja —que hoy
  // significa «dame el foco» para pasar de página— colocaría una firma.
  it("places nothing when the pointer is clicked without dragging", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(<Placing document={document} onPlace={onPlace} />);
    await waitFor(() => expect(renders).toHaveLength(1));

    traceOver(sheet(), [100, 100], [102, 101]);

    expect(onPlace).not.toHaveBeenCalled();
    expect(screen.queryByRole("application")).not.toBeInTheDocument();
  });

  // El síntoma que se ve: sin foco en la hoja, `AvPág` desplazaba la vista en
  // vez de pasar de página (ID-113).
  it("leaves the sheet focused after a bare click, so the page keys still work", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(<Placing document={document} />);
    await waitFor(() => expect(renders).toHaveLength(1));

    traceOver(sheet(), [100, 100], [101, 101]);

    expect(sheet()).toHaveFocus();

    fireEvent.keyDown(sheet(), { key: "PageDown" });

    await waitFor(() => expect(screen.getByLabelText("Número de página")).toHaveValue(2));
  });

  // Trazar es «sellar esta página» con sitio elegido (ID-101): el conjunto se
  // toca igual que con la pastilla, y el rectángulo se mueve en todas las
  // páginas del conjunto porque es un solo campo replicado (ID-96).
  it("adds the traced page to the set and moves the box on every page of it", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <Placing document={document} start={seated} pageChoice="these" onPlace={onPlace} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));
    await goToPage(2, renders);

    traceOver(sheet(), [10, 20], [210, 120]);

    expect(onPlace).toHaveBeenCalledWith({
      rect: toUserSpace(viewportAt(1), { x: 10, y: 20, width: 200, height: 100 }),
      pages: { only: [1, 2] },
    });
  });

  it("replaces the page instead of adding it when only one page is signed", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <Placing document={document} start={seated} pageChoice="single" onPlace={onPlace} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));
    await goToPage(3, renders);

    traceOver(sheet(), [10, 20], [210, 120]);

    expect(onPlace).toHaveBeenCalledWith({
      rect: toUserSpace(viewportAt(1), { x: 10, y: 20, width: 200, height: 100 }),
      pages: { only: [3] },
    });
  });

  it("keeps the whole document sealed when every page is", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <Placing
        document={document}
        start={{ rect: seated.rect, pages: "all" }}
        pageChoice="all"
        onPlace={onPlace}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    traceOver(sheet(), [10, 20], [210, 120]);

    expect(onPlace).toHaveBeenCalledWith({
      rect: toUserSpace(viewportAt(1), { x: 10, y: 20, width: 200, height: 100 }),
      pages: "all",
    });
  });

  // El gesto grueso y el ajuste fino con las flechas (ID-115) son un solo
  // movimiento: tabular hasta el recuadro que acabas de dibujar sobra.
  it("hands the keyboard focus to the box it has just drawn", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(<Placing document={document} />);
    await waitFor(() => expect(renders).toHaveLength(1));

    traceOver(sheet(), [100, 100], [300, 200]);

    await waitFor(() => expect(box()).toHaveFocus());
  });

  // Agarrar el recuadro es moverlo, no trazar encima: los dos gestos escuchan
  // el mismo `pointerdown` porque el recuadro vive dentro de la hoja.
  it("moves the box instead of tracing when the gesture starts on it", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(<Placing document={document} start={seated} onPlace={onPlace} />);
    await waitFor(() => expect(renders).toHaveLength(1));

    const pixels = toPixels(viewportAt(1), seated.rect);
    fireEvent.pointerDown(box(), { pointerId: 1, button: 0, clientX: 100, clientY: 100 });
    fireEvent.pointerMove(box(), { pointerId: 1, clientX: 130, clientY: 140 });
    fireEvent.pointerUp(box(), { pointerId: 1, clientX: 130, clientY: 140 });

    expect(onPlace).toHaveBeenCalledWith({
      rect: toUserSpace(viewportAt(1), movedBy(pixels, 30, 40)),
      pages: seated.pages,
    });
  });

  it("traces nothing while the visible signature cannot be placed", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={null}
        canPlace={false}
        onPlace={onPlace}
        onOpen={noop}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    traceOver(sheet(), [100, 100], [300, 200]);

    expect(onPlace).not.toHaveBeenCalled();
  });
});
