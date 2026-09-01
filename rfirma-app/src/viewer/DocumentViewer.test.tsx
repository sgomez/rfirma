import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { DocumentViewer } from "./DocumentViewer";
import type { PdfDocument, PdfPage, RenderTask, Viewport } from "./pdf";
import type { SignaturePlacement } from "./signatureBox";

/**
 * **Grada A** (`vitest`, carril rápido). Sub-issue #58.
 *
 * `pdf.js` no cabe en `jsdom` —no hay contexto `2d`—, así que el documento
 * entra por el puerto de `pdf.ts` y aquí se enchufa un doble que registra lo
 * que le piden: qué páginas, a qué escala y cuáles se cancelaron.
 */

const A4 = { width: 595, height: 842 };

/** Un viewport con la transformación de `pdf.js` sin rotación: escala y voltea. */
function viewportAt(scale: number): Viewport {
  return {
    width: A4.width * scale,
    height: A4.height * scale,
    convertToPdfPoint: (x, y) => [x / scale, A4.height - y / scale],
    convertToViewportPoint: (x, y) => [x * scale, (A4.height - y) * scale],
  };
}

interface Recorder {
  document: PdfDocument;
  /** Las pintadas lanzadas, en orden. */
  renders: Array<{ page: number; scale: number; cancelled: boolean; finish: () => void }>;
}

/** Un documento cuyas pintadas no acaban solas: las termina la prueba. */
function recordingDocument(pageCount = 3): Recorder {
  const renders: Recorder["renders"] = [];

  const pageOf = (number: number): PdfPage => ({
    number,
    rotate: 0,
    view: [0, 0, 595, 842],
    getViewport: ({ scale }) => viewportAt(scale),
    render: ({ viewport }) => {
      let settle: () => void = () => {};
      let fail: (error: unknown) => void = () => {};
      const entry = {
        page: number,
        scale: viewport.width / A4.width,
        cancelled: false,
        finish: () => settle(),
      };
      renders.push(entry);
      const task: RenderTask = {
        promise: new Promise<void>((resolve, reject) => {
          settle = resolve;
          fail = reject;
        }),
        cancel: () => {
          entry.cancelled = true;
          const cancellation = new Error("Rendering cancelled");
          cancellation.name = "RenderingCancelledException";
          fail(cancellation);
        },
      };
      return task;
    },
  });

  return {
    renders,
    document: { pageCount, getPage: (number) => Promise.resolve(pageOf(number)) },
  };
}

const noop = () => {};

function box() {
  return screen.getByRole("application", { name: "Recuadro de la firma visible" });
}

describe("el visor vacío", () => {
  it("offers the way in, and no floating bar", () => {
    const onOpen = vi.fn();
    renderWithCatalog(
      <DocumentViewer pdf={null} placement={null} onPlace={noop} onOpen={onOpen} />,
    );

    fireEvent.click(screen.getByRole("button", { name: /Arrastra un PDF/ }));

    expect(onOpen).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: "Acercar" })).not.toBeInTheDocument();
    expect(screen.getByText(/no sale de tu ordenador/)).toBeInTheDocument();
  });

  // TD-11: lo que se afirma es lo que se ve, no el CSS calculado. La zona de
  // soltar del artboard tiene tres piezas y las tres tienen que estar.
  it("draws an icon, a title and a supporting line in the drop zone", () => {
    renderWithCatalog(<DocumentViewer pdf={null} placement={null} onPlace={noop} onOpen={noop} />);

    const dropZone = screen.getByRole("button", { name: /Arrastra un PDF/ });

    expect(dropZone.querySelector("svg")).not.toBeNull();
    expect(dropZone).toHaveTextContent("Arrastra un PDF o pulsa para abrirlo");
    expect(dropZone).toHaveTextContent("Se abrirá el explorador de archivos");
  });
});

describe("el visor con documento", () => {
  /**
   * El segundo PDF que no se deja abrir deja el primero en pantalla. Si el
   * aviso solo se pintara en la rama del visor vacío, ese rechazo sería mudo y
   * la pulsación parecería no haber hecho nada.
   */
  it("still tells why a document was refused while another one is painted", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={null}
        onPlace={noop}
        onOpen={noop}
        failure={{ situation: "documentUnreadable", detail: "roto" }}
      />,
    );

    await waitFor(() => expect(renders).toHaveLength(1));
    expect(screen.getByText("No hemos podido leer el documento")).toBeInTheDocument();
  });

  it("renders the first page and counts the rest", async () => {
    const { document, renders } = recordingDocument(27);
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );

    await waitFor(() => expect(renders).toHaveLength(1));
    expect(renders[0]?.page).toBe(1);
    expect(screen.getByLabelText("Número de página")).toHaveValue(1);
    expect(screen.getByText("de 27")).toBeInTheDocument();
  });

  it("cancels the render in flight when the zoom changes", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.click(screen.getByRole("button", { name: "Acercar" }));

    // La primera pintada se cancela: si no, las dos escalas se mezclan sobre el
    // mismo lienzo.
    await waitFor(() => expect(renders[0]?.cancelled).toBe(true));
    await waitFor(() => expect(renders).toHaveLength(2));
    expect(renders[1]?.scale).toBeGreaterThan(1);
    expect(renders[1]?.cancelled).toBe(false);
  });

  it("cancels the render in flight when the page changes", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.click(screen.getByRole("button", { name: "Página siguiente" }));

    await waitFor(() => expect(renders[0]?.cancelled).toBe(true));
    await waitFor(() => expect(renders[1]?.page).toBe(2));
  });

  it("walks to the last page and stops there", async () => {
    const { document, renders } = recordingDocument(3);
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.click(screen.getByRole("button", { name: "Última página" }));
    await waitFor(() => expect(screen.getByLabelText("Número de página")).toHaveValue(3));

    expect(screen.getByRole("button", { name: "Página siguiente" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Primera página" })).toBeEnabled();
  });
});

describe("el recuadro de la firma", () => {
  it("places a first box in user space when a document opens", async () => {
    const onPlace = vi.fn();
    const { document } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={onPlace} onOpen={noop} />,
    );

    await waitFor(() => expect(onPlace).toHaveBeenCalled());
    const placed = onPlace.mock.calls[0]?.[0] as SignaturePlacement;
    expect(placed.page).toBe(1);
    // En puntos del documento, no en píxeles: cabe en la A4 de la prueba.
    expect(placed.rect.x1).toBeLessThanOrEqual(A4.width);
    expect(placed.rect.y1).toBeLessThanOrEqual(A4.height);
  });

  it("keeps the box still over the document when the zoom changes", async () => {
    const placement: SignaturePlacement = { page: 1, rect: { x0: 50, y0: 60, x1: 250, y1: 140 } };
    const { document, renders } = recordingDocument();
    const onPlace = vi.fn();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={placement} onPlace={onPlace} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    expect(box().style.width).toBe("200px");
    expect(box().style.left).toBe("50px");

    fireEvent.click(screen.getByRole("button", { name: "Acercar" }));
    await waitFor(() => expect(renders).toHaveLength(2));

    const scale = renders[1]?.scale ?? 0;
    // Los píxeles siguen al zoom…
    expect(box().style.width).toBe(`${200 * scale}px`);
    // …y el recuadro guardado no se ha tocado: el zoom no lo mueve.
    expect(onPlace).not.toHaveBeenCalled();
  });

  it("stores the drop in user space, converted by the viewport", async () => {
    const placement: SignaturePlacement = { page: 1, rect: { x0: 50, y0: 60, x1: 250, y1: 140 } };
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={placement} onPlace={onPlace} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.pointerDown(box(), { pointerId: 1, button: 0, clientX: 100, clientY: 100 });
    fireEvent.pointerMove(box(), { pointerId: 1, clientX: 110, clientY: 120 });
    fireEvent.pointerUp(box(), { pointerId: 1 });

    // A escala 1 y sin rotación: +10 en X y −20 en Y del documento.
    expect(onPlace).toHaveBeenCalledWith({
      page: 1,
      rect: { x0: 60, y0: 40, x1: 260, y1: 120 },
    });
  });

  it("refuses a drop that falls off the page instead of taking it silently", async () => {
    const placement: SignaturePlacement = { page: 1, rect: { x0: 50, y0: 60, x1: 250, y1: 140 } };
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={placement} onPlace={onPlace} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.pointerDown(box(), { pointerId: 1, button: 0, clientX: 100, clientY: 100 });
    fireEvent.pointerMove(box(), { pointerId: 1, clientX: 900, clientY: 100 });
    fireEvent.pointerUp(box(), { pointerId: 1 });

    expect(onPlace).not.toHaveBeenCalled();
    expect(await screen.findByRole("alert")).toHaveTextContent(/fuera de la página/);
  });

  it("moves with the arrow keys, for whoever is not using a mouse", async () => {
    const placement: SignaturePlacement = { page: 1, rect: { x0: 50, y0: 60, x1: 250, y1: 140 } };
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={placement} onPlace={onPlace} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.keyDown(box(), { key: "ArrowRight" });

    expect(onPlace).toHaveBeenCalledWith({
      page: 1,
      rect: { x0: 51, y0: 60, x1: 251, y1: 140 },
    });
  });
});
