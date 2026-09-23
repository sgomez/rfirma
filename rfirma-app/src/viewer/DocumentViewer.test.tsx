import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { DocumentViewer } from "./DocumentViewer";
import { movedBy, type Placement, toPixels, toUserSpace } from "./signatureBox";
import {
  A4,
  box,
  noop,
  type Recorder,
  recordingDocument,
  seated,
  sheet,
  stubResizeObserver,
  surfaceOf,
  viewportAt,
} from "./testing/documentViewerFixtures";

/**
 * **Grada A** (`vitest`, carril rápido). Sub-issue #58.
 *
 * El recorrido, el zoom y el modo de ajuste. Los tres gestos del recuadro
 * están en `DocumentViewerBox.test.tsx` y `DocumentViewerTrace.test.tsx`, y el
 * sello dentro del recuadro en `DocumentViewerStamp.test.tsx`.
 */

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
      const placed = onPlace.mock.calls[0]?.[0] as Placement;

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

  it("is broken by a zoom fixed by hand, and the next document keeps it (ID-117 enmendado)", async () => {
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

    // El porcentaje fijado a mano sobrevive al documento siguiente: manda lo
    // último que dijo la persona usuaria, no el ajuste de partida.
    await waitFor(() => expect(second.renders).toHaveLength(1));
    expect(second.renders[0]?.scale).toBe(1.25);
  });

  it("opens a fresh document fitted to the whole page, tighter axis first", async () => {
    const observer = stubResizeObserver();
    const { document, renders } = recordingDocument();
    const { container } = renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    observer.resizeTo(surfaceOf(container), 800, 400);

    // Sin tocar nada: el ajuste de partida ya es «a la página», no un
    // porcentaje libre (ID-117 enmendado).
    await waitFor(() => expect(latest(renders)?.scale).toBeCloseTo((400 * 0.92) / A4.height));
  });

  it("keeps fitting the page when the surface opens narrow and tall as well", async () => {
    const observer = stubResizeObserver();
    const { document, renders } = recordingDocument();
    const { container } = renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    observer.resizeTo(surfaceOf(container), 400, 900);

    await waitFor(() => expect(latest(renders)?.scale).toBeCloseTo((400 * 0.92) / A4.width));
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
describe("la página que se está mirando", () => {
  it("tells which page is being looked at, so the panel does not say the wrong one", async () => {
    const onPageChange = vi.fn();
    const { document, renders } = recordingDocument(3);
    renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={null}
        onPlace={noop}
        onOpen={noop}
        onPageChange={onPageChange}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));
    expect(onPageChange).toHaveBeenCalledWith(1);

    fireEvent.click(screen.getByRole("button", { name: "Página siguiente" }));

    await waitFor(() => expect(onPageChange).toHaveBeenLastCalledWith(2));
  });
});

function latest(renders: Recorder["renders"]) {
  return renders[renders.length - 1];
}

/**
 * #190: lo que apaga el bloque del panel —el interruptor en «no», o no haber
 * elegido certificado (ID-108)— tiene que apagar **también el visor**. Hasta
 * aquí el visor no lo miraba, así que la pastilla seguía ofreciendo sellar y el
 * recuadro seguía pintado sobre la hoja.
 */
describe("el visor cuando no se puede colocar la firma visible", () => {
  it("ignores a seal request, since the panel should not have offered the button either", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    const { rerender } = renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={null}
        canPlace={false}
        onPlace={onPlace}
        onOpen={noop}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    rerender(
      <DocumentViewer
        pdf={document}
        placement={null}
        canPlace={false}
        onPlace={onPlace}
        onOpen={noop}
        placementRequest={{ action: "seal" }}
      />,
    );

    expect(onPlace).not.toHaveBeenCalled();
  });

  // La colocación **no se borra** al apagar (así lo hace ya el panel): lo que
  // desaparece es el recuadro, y vuelve intacto al reencender.
  it("paints no box over the sheet, and paints it again when switched back on", async () => {
    const { document, renders } = recordingDocument();
    const { rerender } = renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={seated}
        canPlace={false}
        onPlace={noop}
        onOpen={noop}
      />,
    );

    await waitFor(() => expect(renders).toHaveLength(1));
    expect(screen.queryByRole("application")).not.toBeInTheDocument();

    rerender(
      <DocumentViewer pdf={document} placement={seated} canPlace onPlace={noop} onOpen={noop} />,
    );

    expect(box()).toBeInTheDocument();
  });
});

/**
 * #190: de los tres caminos que colocan la firma, el trazo es el único que
 * elige sitio, y era el único que no funcionaba.
 */
