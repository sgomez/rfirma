import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { DocumentViewer } from "./DocumentViewer";
import type { Placement } from "./signatureBox";
import {
  A4,
  box,
  goToPage,
  noop,
  recordingDocument,
  seated,
} from "./testing/documentViewerFixtures";

/**
 * **Grada A** (`vitest`, carril rápido). Sub-issue #58.
 *
 * Los dos gestos que no trazan: arrastrar/redimensionar el recuadro y la
 * petición de sellar o quitar el sello desde el panel. El trazo, en
 * `DocumentViewerTrace.test.tsx`, y el recorrido/zoom, en `DocumentViewer.test.tsx`.
 */

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
    const remembered: Placement = {
      rect: { x0: 50, y0: 60, x1: 250, y1: 140 },
      pages: { only: [3] },
    };
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
    const remembered: Placement = {
      rect: { x0: 50, y0: 60, x1: 250, y1: 140 },
      pages: { only: [9] },
    };
    const { document, renders } = recordingDocument(3);
    const { rerender } = renderWithCatalog(
      <DocumentViewer pdf={null} placement={null} onPlace={noop} onOpen={noop} />,
    );

    rerender(<DocumentViewer pdf={document} placement={remembered} onPlace={noop} onOpen={noop} />);

    await waitFor(() => expect(renders).toHaveLength(1));
    expect(renders[0]?.page).toBe(3);
  });

  it("keeps the box still over the document when the zoom changes", async () => {
    const placement: Placement = {
      rect: { x0: 50, y0: 60, x1: 250, y1: 140 },
      pages: { only: [1] },
    };
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
    const placement: Placement = {
      rect: { x0: 50, y0: 60, x1: 250, y1: 140 },
      pages: { only: [1] },
    };
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
      pages: { only: [1] },
      rect: { x0: 60, y0: 40, x1: 260, y1: 120 },
    });
  });

  it("refuses a drop that falls off the page instead of taking it silently", async () => {
    const placement: Placement = {
      rect: { x0: 50, y0: 60, x1: 250, y1: 140 },
      pages: { only: [1] },
    };
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
    const placement: Placement = {
      rect: { x0: 50, y0: 60, x1: 250, y1: 140 },
      pages: { only: [1] },
    };
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={placement} onPlace={onPlace} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.keyDown(box(), { key: "ArrowRight" });

    expect(onPlace).toHaveBeenCalledWith({
      pages: { only: [1] },
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
      pages: { only: [1] },
      rect: { x0: 51, y0: 60, x1: 251, y1: 140 },
    });
  });
});

/** ID-107, ID-109: dentro del recuadro va el sello de verdad, o no hay recuadro. */
describe("el conjunto de páginas en la hoja", () => {
  const onTwo: Placement = {
    rect: { x0: 50, y0: 60, x1: 250, y1: 140 },
    pages: { only: [1, 3] },
  };

  it("draws the same box on every page of the set, and no differently on any of them", async () => {
    const { document, renders } = recordingDocument(3);
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={onTwo} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));
    const first = { left: box().style.left, top: box().style.top, width: box().style.width };

    await goToPage(3, renders);

    expect(box().style.left).toBe(first.left);
    expect(box().style.top).toBe(first.top);
    expect(box().style.width).toBe(first.width);
  });

  /**
   * El criterio 2 del #174: sellada la 3 y estando en la 7, la 7 se ve **en
   * blanco**. Ni recuadro ni fantasma a trazos: un fantasma insinúa que ahí hay
   * algo, que es la mentira que este hito existe para quitar.
   */
  it("leaves a page outside the set blank, with no dashed ghost either", async () => {
    const { document, renders } = recordingDocument(9);
    const { container } = renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={{ rect: { x0: 50, y0: 60, x1: 250, y1: 140 }, pages: { only: [3] } }}
        onPlace={noop}
        onOpen={noop}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    await goToPage(7, renders);

    expect(container.querySelectorAll(".viewer__box")).toHaveLength(0);
    expect(container.querySelectorAll(".viewer__grip")).toHaveLength(0);
  });

  it("draws the box on every page when the whole document is stamped", async () => {
    const { document, renders } = recordingDocument(3);
    renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={{ rect: { x0: 50, y0: 60, x1: 250, y1: 140 }, pages: "all" }}
        onPlace={noop}
        onOpen={noop}
        pageChoice="all"
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    await goToPage(2, renders);

    expect(box()).toBeInTheDocument();
  });

  /** Mover el recuadro en una página lo mueve en todas: es un widget replicado. */
  it("keeps the set untouched when the box is dragged", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument(3);
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={onTwo} onPlace={onPlace} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.keyDown(box(), { key: "ArrowRight" });

    const [placed] = onPlace.mock.calls[0] as [Placement];
    expect(placed.pages).toEqual({ only: [1, 3] });
  });
});

/** ID-104: los tiradores son cromo, no papel. */
describe("los tiradores del recuadro", () => {
  it("measures the same on screen at 50 %, 100 % and 300 %", async () => {
    const { document, renders } = recordingDocument();
    const { container } = renderWithCatalog(
      <DocumentViewer pdf={document} placement={seated} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    const sides = [];
    for (const zoom of ["50", "100", "300"]) {
      fireEvent.change(screen.getByLabelText("Nivel de zoom"), { target: { value: zoom } });
      fireEvent.keyDown(screen.getByLabelText("Nivel de zoom"), { key: "Enter" });
      await waitFor(() => expect(renders.at(-1)?.scale).toBe(Number(zoom) / 100));
      const grip = container.querySelector(".viewer__grip") as HTMLElement;
      sides.push([grip.style.width, grip.style.height]);
    }

    expect(sides).toEqual([
      ["10px", "10px"],
      ["10px", "10px"],
      ["10px", "10px"],
    ]);
  });

  it("hangs one grip on each of the four corners", async () => {
    const { document, renders } = recordingDocument();
    const { container } = renderWithCatalog(
      <DocumentViewer pdf={document} placement={seated} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    expect(
      [...container.querySelectorAll(".viewer__grip")].map((g) => g.getAttribute("data-corner")),
    ).toEqual(["top-left", "top-right", "bottom-left", "bottom-right"]);
  });
});

/** ID-101 e ID-102: la pastilla bajo la hoja, sus tres caras y su cuarta redacción. */
/**
 * El botón de sellar vive en el panel desde #194; el visor solo atiende la
 * petición que cruza por `placementRequest`, porque es quien tiene el
 * `viewport` que mide la posición estándar del recuadro.
 */
describe("la petición de sellar o quitar el sello", () => {
  it("places a document that has none, on this page, at the standard position", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    const { rerender } = renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={onPlace} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    rerender(
      <DocumentViewer
        pdf={document}
        placement={null}
        onPlace={onPlace}
        onOpen={noop}
        placementRequest={{ action: "seal" }}
      />,
    );

    const [placed] = onPlace.mock.calls[0] as [Placement];
    expect(placed.pages).toEqual({ only: [1] });
    // La posición estándar: abajo a la derecha, dentro de la página (ID-102).
    expect(placed.rect.x1).toBeGreaterThan(A4.width / 2);
    expect(placed.rect.x1).toBeLessThanOrEqual(A4.width);
  });

  it("adds the page it is looking at to a placement that already exists", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument(3);
    const { rerender } = renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={{ rect: { x0: 50, y0: 60, x1: 250, y1: 140 }, pages: { only: [1] } }}
        onPlace={onPlace}
        onOpen={noop}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));
    await goToPage(2, renders);

    rerender(
      <DocumentViewer
        pdf={document}
        placement={{ rect: { x0: 50, y0: 60, x1: 250, y1: 140 }, pages: { only: [1] } }}
        onPlace={onPlace}
        onOpen={noop}
        placementRequest={{ action: "seal" }}
      />,
    );

    expect(onPlace).toHaveBeenCalledWith({
      rect: { x0: 50, y0: 60, x1: 250, y1: 140 },
      pages: { only: [1, 2] },
    });
  });

  /** ID-92: quitar la última página devuelve al estado del PDF recién abierto. */
  it("takes the whole placement away with the last page of the set", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    const { rerender } = renderWithCatalog(
      <DocumentViewer pdf={document} placement={seated} onPlace={onPlace} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    rerender(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={onPlace}
        onOpen={noop}
        placementRequest={{ action: "unseal" }}
      />,
    );

    expect(onPlace).toHaveBeenCalledWith(null);
  });

  /**
   * Toda la razón de que cada petición sea un objeto nuevo: pulsar el botón
   * del panel dos veces tiene que actuar las dos veces, aunque la acción no
   * haya cambiado.
   */
  it("acts again when asked for the same action twice", async () => {
    const onPlace = vi.fn();
    const { document, renders } = recordingDocument();
    const { rerender } = renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={onPlace} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    rerender(
      <DocumentViewer
        pdf={document}
        placement={null}
        onPlace={onPlace}
        onOpen={noop}
        placementRequest={{ action: "seal" }}
      />,
    );
    expect(onPlace).toHaveBeenCalledTimes(1);

    rerender(
      <DocumentViewer
        pdf={document}
        placement={null}
        onPlace={onPlace}
        onOpen={noop}
        placementRequest={{ action: "seal" }}
      />,
    );
    expect(onPlace).toHaveBeenCalledTimes(2);
  });
});

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

/**
 * El panel necesita saber por dónde va el visor para elegir la cara del botón
 * de sellar (#194, antes ID-100).
 */
