import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { DocumentViewer } from "./DocumentViewer";
import type { PdfDocument, PdfPage, RenderTask, Viewport } from "./pdf";
import type { SignaturePlacement } from "./signatureBox";
import { movedBy, toPixels, toUserSpace } from "./signatureBox";

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

/**
 * Un recuadro ya colocado. Desde el ID-114 el visor **no lo crea**: quien lo
 * quiera en pantalla lo entrega, igual que hace la fila que lo recuerda.
 */
const seated: SignaturePlacement = { page: 1, rect: { x0: 50, y0: 60, x1: 250, y1: 140 } };

function box() {
  return screen.getByRole("application", { name: "Recuadro de la firma visible" });
}

function sheet() {
  return screen.getByRole("document", { name: "Hoja del documento" });
}

/** La parte visible del visor, la que se mide para ajustar. */
function surfaceOf(container: HTMLElement): HTMLElement {
  return container.querySelector(".viewer__scroll") as HTMLElement;
}

/**
 * `jsdom` no trae `ResizeObserver` ni mide nada, así que la parte visible se
 * dimensiona a mano y el redimensionado se dispara desde la prueba.
 */
function stubResizeObserver() {
  let notify: () => void = () => {};
  class Stub {
    constructor(callback: () => void) {
      notify = callback;
    }
    observe() {}
    disconnect() {}
    unobserve() {}
  }
  vi.stubGlobal("ResizeObserver", Stub);
  return {
    resizeTo(element: HTMLElement, width: number, height: number) {
      Object.defineProperty(element, "clientWidth", { value: width, configurable: true });
      Object.defineProperty(element, "clientHeight", { value: height, configurable: true });
      act(() => notify());
    },
  };
}

afterEach(() => vi.unstubAllGlobals());

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

  /**
   * ID-84: el mapa de bits sale al doble de resolución que el `<canvas>` en
   * píxeles CSS, en una pantalla 2x.
   */
  it("rasterises at devicePixelRatio, twice the CSS size on a 2x screen", async () => {
    const original = window.devicePixelRatio;
    Object.defineProperty(window, "devicePixelRatio", { value: 2, configurable: true });
    try {
      const { document, renders } = recordingDocument();
      const { container } = renderWithCatalog(
        <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
      );

      await waitFor(() => expect(renders).toHaveLength(1));
      const canvas = container.querySelector("canvas") as HTMLCanvasElement;
      expect(canvas.width).toBe(A4.width * 2);
      expect(canvas.height).toBe(A4.height * 2);
      expect(canvas.style.width).toBe(`${A4.width}px`);
      expect(canvas.style.height).toBe(`${A4.height}px`);
    } finally {
      Object.defineProperty(window, "devicePixelRatio", { value: original, configurable: true });
    }
  });

  /**
   * La nitidez es cosa del mapa de bits: el viewport que se usa para convertir
   * el recuadro a espacio de usuario PDF sigue en píxeles CSS, así que el
   * `/Rect` que acaba en el PDF no se mueve por la pantalla en la que se firmó
   * (ID-84).
   *
   * La caja por omisión es proporcional a la página y la conversión a espacio
   * de usuario divide por la escala del viewport, así que es invariante de
   * escala por sí sola: no distinguiría un viewport en píxeles CSS de uno en
   * píxeles de mapa de bits. La prueba mueve el recuadro con las flechas —el
   * mismo camino que usa el arrastre, `toUserSpace(viewport, moved)`— un
   * desplazamiento fijo **en píxeles CSS** y afirma los puntos PDF exactos
   * que resultarían con el viewport correcto (escala 1, no 2).
   */
  it("keeps the box-to-user-space conversion in CSS pixels regardless of devicePixelRatio", async () => {
    const original = window.devicePixelRatio;
    Object.defineProperty(window, "devicePixelRatio", { value: 2, configurable: true });
    try {
      const onPlace = vi.fn();
      const { document, renders } = recordingDocument();
      renderWithCatalog(
        <DocumentViewer pdf={document} placement={seated} onPlace={onPlace} onOpen={noop} />,
      );

      await waitFor(() => expect(renders).toHaveLength(1));

      fireEvent.keyDown(box(), { key: "ArrowRight", shiftKey: true });

      await waitFor(() => expect(onPlace).toHaveBeenCalled());
      const placed = onPlace.mock.calls[0]?.[0] as SignaturePlacement;

      // El mismo cálculo con el viewport de píxeles CSS (escala 1), nunca el
      // del mapa de bits (escala `devicePixelRatio`): si `toUserSpace` se
      // rompiera y empezara a usar el viewport equivocado, este valor
      // esperado ya no coincidiría con lo que produce el componente.
      const moved = movedBy(toPixels(viewportAt(1), seated.rect), 10, 0);
      expect(placed.rect).toEqual(toUserSpace(viewportAt(1), moved));
    } finally {
      Object.defineProperty(window, "devicePixelRatio", { value: original, configurable: true });
    }
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
  /**
   * ID-114: abrir un documento **no** escribe en la colocación. El visor ya no
   * siembra un recuadro; el que haya llega por la prop, y así un redondeo a un
   * zoom raro no puede reescribir la fila guardada del documento (ID-74).
   */
  it("does not write the placement when a document opens", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={onPlace} onOpen={noop} />,
    );

    await waitFor(() => expect(renders).toHaveLength(1));
    expect(onPlace).not.toHaveBeenCalled();
    expect(
      screen.queryByRole("application", { name: "Recuadro de la firma visible" }),
    ).not.toBeInTheDocument();
  });

  /** ID-113 con el ID-96: en otra página no hay recuadro, así que `Tab` no lo alcanza. */
  it("shows the box only on its own page", async () => {
    const { document, renders } = recordingDocument(3);
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={seated} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));
    expect(box()).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Página siguiente" }));

    await waitFor(() => expect(renders).toHaveLength(2));
    expect(
      screen.queryByRole("application", { name: "Recuadro de la firma visible" }),
    ).not.toBeInTheDocument();
  });

  /**
   * El criterio del #126: reabrir un documento **repone su página**. El visor
   * arrancaba siempre en la 1, así que el efecto de colocación veía que no
   * coincidía con la página guardada y la pisaba con la 1 a través de
   * `onPlace` —que ahora escribe en la fila—.
   */
  it("opens on the page the row remembered instead of resetting it to the first", async () => {
    const remembered: SignaturePlacement = { page: 3, rect: { x0: 50, y0: 60, x1: 250, y1: 140 } };
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument(5);
    const { rerender } = renderWithCatalog(
      <DocumentViewer pdf={null} placement={null} onPlace={onPlace} onOpen={noop} />,
    );

    rerender(
      <DocumentViewer pdf={document} placement={remembered} onPlace={onPlace} onOpen={noop} />,
    );

    await waitFor(() => expect(renders).toHaveLength(1));
    expect(renders[0]?.page).toBe(3);
    expect(screen.getByLabelText("Número de página")).toHaveValue(3);
    expect(onPlace).not.toHaveBeenCalled();
  });

  /** Una fila vieja con una página que el documento ya no tiene no lo rompe. */
  it("clamps a remembered page that the document no longer has", async () => {
    const remembered: SignaturePlacement = { page: 9, rect: { x0: 50, y0: 60, x1: 250, y1: 140 } };
    const { document, renders } = recordingDocument(3);
    const { rerender } = renderWithCatalog(
      <DocumentViewer pdf={null} placement={null} onPlace={noop} onOpen={noop} />,
    );

    rerender(<DocumentViewer pdf={document} placement={remembered} onPlace={noop} onOpen={noop} />);

    await waitFor(() => expect(renders).toHaveLength(1));
    expect(renders[0]?.page).toBe(3);
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

  /**
   * ID-115: el empuje es de **un punto de espacio de usuario**, no de un píxel
   * del lienzo. Al 200 % una flecha seguía moviendo medio punto, así que
   * colocar con precisión obligaba a acercarse primero.
   */
  it("nudges by a user-space point, whatever the zoom", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={seated} onPlace={onPlace} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.change(screen.getByLabelText("Nivel de zoom"), { target: { value: "200" } });
    fireEvent.keyDown(screen.getByLabelText("Nivel de zoom"), { key: "Enter" });
    await waitFor(() => expect(renders[1]?.scale).toBe(2));

    fireEvent.keyDown(box(), { key: "ArrowRight" });

    expect(onPlace).toHaveBeenCalledWith({
      page: 1,
      rect: { x0: 51, y0: 60, x1: 251, y1: 140 },
    });
  });
});

describe("el zoom continuo", () => {
  it("magnifies with Ctrl and the wheel, which is also how the trackpad pinch arrives", async () => {
    const { document, renders } = recordingDocument();
    const { container } = renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.wheel(surfaceOf(container), {
      ctrlKey: true,
      deltaY: -100,
      clientX: 40,
      clientY: 30,
    });

    await waitFor(() => expect(renders).toHaveLength(2));
    expect(renders[1]?.scale).toBeCloseTo(Math.exp(0.5), 6);
  });

  /**
   * React registra `wheel` como oyente **pasivo**, y dentro de uno pasivo
   * `preventDefault()` es un no-op: con la prop `onWheel` el gesto conservaba
   * su acción por defecto y ampliaba el WebView entero además del documento.
   */
  it("cancels the browser's own zoom, which a passive listener could not", async () => {
    const { document, renders } = recordingDocument();
    const { container } = renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    const gesture = new WheelEvent("wheel", {
      ctrlKey: true,
      deltaY: -100,
      bubbles: true,
      cancelable: true,
    });
    act(() => {
      surfaceOf(container).dispatchEvent(gesture);
    });

    expect(gesture.defaultPrevented).toBe(true);
  });

  it("leaves the wheel alone without Ctrl, which is how the document scrolls", async () => {
    const { document, renders } = recordingDocument();
    const { container } = renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.wheel(surfaceOf(container), { deltaY: -100 });

    expect(renders).toHaveLength(1);
  });

  it("takes the percentage typed in the bar, clipped to the range", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));
    const level = screen.getByLabelText("Nivel de zoom");

    fireEvent.change(level, { target: { value: "1000" } });
    fireEvent.keyDown(level, { key: "Enter" });

    await waitFor(() => expect(renders[1]?.scale).toBe(4));
  });

  it("comes back to 100 % with Ctrl+0", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));
    fireEvent.click(screen.getByRole("button", { name: "Acercar" }));
    await waitFor(() => expect(renders).toHaveLength(2));

    fireEvent.keyDown(sheet(), { key: "0", ctrlKey: true });

    await waitFor(() => expect(renders[2]?.scale).toBe(1));
  });

  /** ID-116: los botones ± tropiezan con los siete escalones. */
  it("trips over the steps with the buttons, and reaches the ceiling from the last one", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.click(screen.getByRole("button", { name: "Acercar" }));

    await waitFor(() => expect(renders[1]?.scale).toBe(1.25));
  });
});

describe("«ajustar» como modo", () => {
  it("stays fitted across a resize, a page change and another document", async () => {
    const observer = stubResizeObserver();
    const first = recordingDocument(3);
    const { container, rerender } = renderWithCatalog(
      <DocumentViewer pdf={first.document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(first.renders).toHaveLength(1));
    observer.resizeTo(surfaceOf(container), 800, 600);

    fireEvent.click(screen.getByRole("button", { name: "Ajustar al ancho" }));
    await waitFor(() => expect(latest(first.renders)?.scale).toBeCloseTo((800 * 0.92) / A4.width));

    // La ventana cambia de tamaño: sigue ajustado, que es lo que «ajustar» como
    // modo significa (ID-117).
    observer.resizeTo(surfaceOf(container), 1200, 600);
    await waitFor(() => expect(latest(first.renders)?.scale).toBeCloseTo((1200 * 0.92) / A4.width));

    // Se pasa de página: sigue ajustado.
    fireEvent.click(screen.getByRole("button", { name: "Página siguiente" }));
    await waitFor(() => expect(latest(first.renders)?.page).toBe(2));
    expect(latest(first.renders)?.scale).toBeCloseTo((1200 * 0.92) / A4.width);

    // Y se abre otro documento: el modo cruza, porque describe cómo se mira y
    // no cuánto se amplía *ese* documento.
    const second = recordingDocument(3);
    rerender(
      <DocumentViewer pdf={second.document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(second.renders.length).toBeGreaterThan(0));
    await waitFor(() =>
      expect(latest(second.renders)?.scale).toBeCloseTo((1200 * 0.92) / A4.width),
    );
  });

  /**
   * El visor se monta con `pdf === null` —`App.tsx` no lo condiciona ni le
   * pone `key`—, y la parte visible sólo existe en la rama con documento. Si
   * el observador se enganchara en el montaje, no se engancharía nunca y
   * «ajustar» no ajustaría nada en lo que se instala.
   */
  it("fits a document that arrived after the viewer was already mounted", async () => {
    const observer = stubResizeObserver();
    const { document, renders } = recordingDocument();
    const { container, rerender } = renderWithCatalog(
      <DocumentViewer pdf={null} placement={null} onPlace={noop} onOpen={noop} />,
    );

    rerender(<DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />);
    await waitFor(() => expect(renders).toHaveLength(1));
    observer.resizeTo(surfaceOf(container), 800, 600);

    fireEvent.click(screen.getByRole("button", { name: "Ajustar al ancho" }));

    await waitFor(() => expect(latest(renders)?.scale).toBeCloseTo((800 * 0.92) / A4.width));
  });

  it("fits the whole page when that is what was asked, tighter axis first", async () => {
    const observer = stubResizeObserver();
    const { document, renders } = recordingDocument();
    const { container } = renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));
    observer.resizeTo(surfaceOf(container), 800, 400);

    fireEvent.click(screen.getByRole("button", { name: "Ajustar a la página" }));

    await waitFor(() => expect(latest(renders)?.scale).toBeCloseTo((400 * 0.92) / A4.height));
  });

  it("is broken by a zoom fixed by hand, and then the next document is back at 100 %", async () => {
    const observer = stubResizeObserver();
    const first = recordingDocument();
    const { container, rerender } = renderWithCatalog(
      <DocumentViewer pdf={first.document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(first.renders).toHaveLength(1));
    observer.resizeTo(surfaceOf(container), 800, 600);
    fireEvent.click(screen.getByRole("button", { name: "Ajustar al ancho" }));
    await waitFor(() => expect(latest(first.renders)?.scale).toBeCloseTo((800 * 0.92) / A4.width));

    fireEvent.click(screen.getByRole("button", { name: "Acercar" }));
    // El botón tropieza con el escalón siguiente al ajuste, el 125 %.
    await waitFor(() => expect(latest(first.renders)?.scale).toBe(1.25));
    // Y ya no está ajustado: estirar la ventana no lo mueve.
    observer.resizeTo(surfaceOf(container), 1200, 600);
    expect(latest(first.renders)?.scale).toBe(1.25);

    const second = recordingDocument();
    rerender(
      <DocumentViewer pdf={second.document} placement={null} onPlace={noop} onOpen={noop} />,
    );

    await waitFor(() => expect(second.renders).toHaveLength(1));
    expect(second.renders[0]?.scale).toBe(1);
  });

  /** ID-114: ni el zoom ni el redimensionado escriben en la colocación. */
  it("writes nothing to the placement while zooming and resizing", async () => {
    const observer = stubResizeObserver();
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument(3);
    const { container } = renderWithCatalog(
      <DocumentViewer pdf={document} placement={seated} onPlace={onPlace} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    observer.resizeTo(surfaceOf(container), 800, 600);
    fireEvent.click(screen.getByRole("button", { name: "Ajustar al ancho" }));
    fireEvent.click(screen.getByRole("button", { name: "Acercar" }));
    fireEvent.click(screen.getByRole("button", { name: "Página siguiente" }));
    await waitFor(() => expect(renders.length).toBeGreaterThan(1));

    expect(onPlace).not.toHaveBeenCalled();
  });
});

describe("el reparto del foco", () => {
  it("turns the pages with the focus on the sheet", async () => {
    const { document, renders } = recordingDocument(5);
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.keyDown(sheet(), { key: "PageDown" });
    await waitFor(() => expect(screen.getByLabelText("Número de página")).toHaveValue(2));

    fireEvent.keyDown(sheet(), { key: "End" });
    await waitFor(() => expect(screen.getByLabelText("Número de página")).toHaveValue(5));

    fireEvent.keyDown(sheet(), { key: "Home" });
    await waitFor(() => expect(screen.getByLabelText("Número de página")).toHaveValue(1));
  });

  /**
   * ID-113: las teclas de página **burbujean** desde el recuadro hasta la
   * hoja, así que se pasa de página sin salir del recuadro.
   */
  it("turns the pages from inside the box too, because the keys bubble", async () => {
    const { document, renders } = recordingDocument(5);
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={seated} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.keyDown(box(), { key: "PageDown" });

    await waitFor(() => expect(screen.getByLabelText("Número de página")).toHaveValue(2));
  });

  it("gives the focus back to the sheet with Esc", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={seated} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));
    box().focus();

    fireEvent.keyDown(box(), { key: "Escape" });

    expect(sheet()).toHaveFocus();
  });

  it("makes both the sheet and the box reachable with Tab", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={seated} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    expect(sheet()).toHaveAttribute("tabindex", "0");
    expect(box()).toHaveAttribute("tabindex", "0");
  });
});

describe("el tope del mapa de bits", () => {
  /**
   * ID-119: al 400 % con `devicePixelRatio` 2 el lienzo se pinta a 4×, no a
   * 8×. Serían ~4 760 × 6 736 px y 128 MB para una sola página, y con el
   * porcentaje editable ese techo se alcanza tecleando.
   */
  it("paints at four times and not eight at 400 % on a 2x screen", async () => {
    const original = window.devicePixelRatio;
    Object.defineProperty(window, "devicePixelRatio", { value: 2, configurable: true });
    try {
      const { document, renders } = recordingDocument();
      const { container } = renderWithCatalog(
        <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
      );
      await waitFor(() => expect(renders).toHaveLength(1));

      const level = screen.getByLabelText("Nivel de zoom");
      fireEvent.change(level, { target: { value: "400" } });
      fireEvent.keyDown(level, { key: "Enter" });

      await waitFor(() => expect(latest(renders)?.scale).toBe(4));
      const canvas = container.querySelector("canvas") as HTMLCanvasElement;
      expect(canvas.width).toBe(A4.width * 4);
      // El zoom que ve la persona sigue siendo el 400 %: lo recortado es la
      // resolución del lienzo, y por eso el tamaño en CSS no se toca.
      expect(canvas.style.width).toBe(`${A4.width * 4}px`);
    } finally {
      Object.defineProperty(window, "devicePixelRatio", { value: original, configurable: true });
    }
  });
});

/** La última pintada lanzada, que es la que se está mirando. */
describe("el recuadro que se trae a la vista", () => {
  /**
   * ID-118. El recuadro se pinta **sólo en su página**, así que la página de
   * paso no tiene ninguno: si el paso por ella no contara como atendida, el
   * regreso a la página del recuadro —el único caso que esto cubre— saldría
   * por la guarda sin traer nada.
   */
  it("brings the box back into view on returning to its page", async () => {
    const brought = vi.fn();
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
      value: brought,
      configurable: true,
      writable: true,
    });
    // `jsdom` no hace layout: sin esto todo mide cero y el recuadro siempre
    // «se ve». El recuadro se pone lejos, fuera de la parte visible.
    const measure = vi
      .spyOn(HTMLElement.prototype, "getBoundingClientRect")
      .mockImplementation(function (this: HTMLElement) {
        const far = this.classList.contains("viewer__box");
        return {
          top: far ? 5000 : 0,
          bottom: far ? 5100 : 600,
          left: 0,
          right: far ? 100 : 800,
          width: far ? 100 : 800,
          height: far ? 100 : 600,
          x: 0,
          y: far ? 5000 : 0,
          toJSON: () => ({}),
        } as DOMRect;
      });

    try {
      const { document, renders } = recordingDocument(3);
      renderWithCatalog(
        <DocumentViewer pdf={document} placement={seated} onPlace={noop} onOpen={noop} />,
      );
      await waitFor(() => expect(renders).toHaveLength(1));
      expect(brought).not.toHaveBeenCalled();

      fireEvent.click(screen.getByRole("button", { name: "Página siguiente" }));
      await waitFor(() => expect(screen.getByLabelText("Número de página")).toHaveValue(2));

      fireEvent.click(screen.getByRole("button", { name: "Página anterior" }));
      await waitFor(() => expect(screen.getByLabelText("Número de página")).toHaveValue(1));

      await waitFor(() => expect(brought).toHaveBeenCalled());
    } finally {
      measure.mockRestore();
      delete (HTMLElement.prototype as Partial<HTMLElement>).scrollIntoView;
    }
  });
});

function latest(renders: Recorder["renders"]) {
  return renders[renders.length - 1];
}
