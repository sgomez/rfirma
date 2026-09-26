import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { DocumentViewer } from "./DocumentViewer";
import { box, noop, recordingDocument, seated } from "./testing/documentViewerFixtures";

/**
 * **Grada A** (`vitest`, carril rápido). Sub-issue #58.
 *
 * El sello dentro del recuadro y la pastilla flotante que cuenta su estado.
 * Los tres gestos del recuadro, en `DocumentViewerBox.test.tsx` y
 * `DocumentViewerTrace.test.tsx`.
 */

describe("el sello dentro del recuadro", () => {
  it("paints the stamped document instead of the original one", async () => {
    const original = recordingDocument();
    const preview = recordingDocument();

    renderWithCatalog(
      <DocumentViewer
        pdf={original.document}
        stamped={preview.document}
        placement={seated}
        onPlace={noop}
        onOpen={noop}
      />,
    );

    // Cero código de dibujo nuevo: lo que se ve es el PDF en seco, pintado por
    // `pdf.js` de fábrica, y el original no se pinta en absoluto.
    await waitFor(() => expect(preview.renders).toHaveLength(1));
    expect(original.renders).toHaveLength(0);
  });

  it("comes back to the original document when there is no stamp to show", async () => {
    const original = recordingDocument();
    const preview = recordingDocument();
    const { rerender } = renderWithCatalog(
      <DocumentViewer
        pdf={original.document}
        stamped={preview.document}
        placement={seated}
        onPlace={noop}
        onOpen={noop}
      />,
    );
    await waitFor(() => expect(preview.renders).toHaveLength(1));

    rerender(
      <DocumentViewer
        pdf={original.document}
        stamped={null}
        placement={seated}
        onPlace={noop}
        onOpen={noop}
      />,
    );

    await waitFor(() => expect(original.renders).toHaveLength(1));
  });

  it("dims the frozen view while the box is being dragged", async () => {
    const { document, renders } = recordingDocument();
    const { container, rerender } = renderWithCatalog(
      <DocumentViewer pdf={document} placement={seated} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));
    expect(container.querySelector(".viewer__stamp-frozen")).toBeNull();

    rerender(
      <DocumentViewer pdf={document} placement={seated} stampFrozen onPlace={noop} onOpen={noop} />,
    );

    // El atenuado va sobre el recuadro **de antes del gesto**: el que se
    // arrastra se ha ido con el puntero, y lo congelado se queda donde estaba.
    const frozen = container.querySelector(".viewer__stamp-frozen") as HTMLElement;
    expect(frozen.style.left).toBe("50px");
    expect(frozen.style.width).toBe("200px");
  });

  it("tells the gesture apart from the drop, which is when the stamp is recomposed", async () => {
    const onGesture = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={noop}
        onGesture={onGesture}
        onOpen={noop}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.pointerDown(box(), { pointerId: 1, button: 0, clientX: 100, clientY: 100 });
    expect(onGesture).toHaveBeenLastCalledWith(true);

    fireEvent.pointerMove(box(), { pointerId: 1, clientX: 110, clientY: 120 });
    fireEvent.pointerUp(box(), { pointerId: 1 });

    expect(onGesture).toHaveBeenLastCalledWith(false);
  });

  it("says nothing about a gesture the secondary button never started", async () => {
    const onGesture = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={noop}
        onGesture={onGesture}
        onOpen={noop}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    // `useBoxDrag` descarta el botón secundario —es el del menú del sistema—,
    // así que avisar de un gesto aquí congelaría la vista previa sin que se
    // esté moviendo nada.
    fireEvent.pointerDown(box(), { pointerId: 1, button: 2, clientX: 100, clientY: 100 });

    expect(onGesture).not.toHaveBeenCalled();
  });

  it("also freezes while a grip is resizing the box", async () => {
    const onGesture = vi.fn();
    const { document, renders } = recordingDocument();
    const { container } = renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={noop}
        onGesture={onGesture}
        onOpen={noop}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));
    const grip = container.querySelector('[data-corner="bottom-right"]') as HTMLElement;

    fireEvent.pointerDown(grip, { pointerId: 1, button: 0, clientX: 250, clientY: 100 });
    expect(onGesture).toHaveBeenLastCalledWith(true);

    fireEvent.pointerUp(grip, { pointerId: 1 });
    expect(onGesture).toHaveBeenLastCalledWith(false);
  });
});

/**
 * ID-107, ID-108, ID-111, #202: el estado del sello, en la pastilla que flota
 * sobre la botonera. Antes vivía en el panel, con una insignia de estado que
 * se ha retirado; aquí es solo texto y, si hace falta, un botón.
 */
describe("el hueco de la rúbrica sin cargar", () => {
  it("draws a dotted gap beside the text where the missing rubric will go", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={noop}
        onOpen={noop}
        stamp={{ kind: "composed" }}
        rubricGap="beside"
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    const gap = within(box()).getByTitle("Sin rúbrica cargada");
    expect(gap).toHaveClass("viewer__rubric-gap--beside");
  });

  it("fills the box with the gap when the model is rubric only", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={noop}
        onOpen={noop}
        stamp={{ kind: "composing" }}
        rubricGap="fill"
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    expect(within(box()).getByTitle("Sin rúbrica cargada")).toHaveClass("viewer__rubric-gap--fill");
  });

  it("leaves the box empty when there is no certificate to compose with", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={noop}
        onOpen={noop}
        stamp={{ kind: "noCertificate" }}
        rubricGap="beside"
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    expect(within(box()).queryByTitle("Sin rúbrica cargada")).not.toBeInTheDocument();
  });
});

describe("el estado del sello, flotando sobre la botonera", () => {
  function stampPill() {
    return screen.queryByRole("status");
  }

  it("mounts no pill when there is nothing to say about the stamp", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer pdf={document} placement={null} onPlace={noop} onOpen={noop} />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    expect(stampPill()).not.toBeInTheDocument();
  });

  it("says nothing once the stamp is up to date", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={noop}
        onOpen={noop}
        stamp={{ kind: "composed" }}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    expect(stampPill()).not.toBeInTheDocument();
  });

  it("says the frozen view is the previous one while the box is being moved", async () => {
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={noop}
        onOpen={noop}
        stamp={{ kind: "frozen" }}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    expect(screen.getByText("Sello congelado mientras mueves el recuadro")).toBeInTheDocument();
  });

  it("asks for the recomposition by hand on a large document", async () => {
    const onComposeStamp = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={noop}
        onOpen={noop}
        stamp={{ kind: "onDemand" }}
        onComposeStamp={onComposeStamp}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    fireEvent.click(screen.getByRole("button", { name: "Ver cómo queda" }));

    expect(onComposeStamp).toHaveBeenCalled();
  });

  /**
   * ID-111. La vista previa **no es una puerta**: sobre si se puede firmar
   * manda el botón de firmar, que no vive aquí.
   */
  it("says it could not draw the stamp, with a way to retry", async () => {
    const onComposeStamp = vi.fn();
    const { document, renders } = recordingDocument();
    renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={noop}
        onOpen={noop}
        stamp={{
          kind: "failed",
          failure: { situation: "documentUnreadable", detail: "el documento tiene contraseña" },
        }}
        onComposeStamp={onComposeStamp}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    expect(screen.getByText("No se ha podido dibujar el sello")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Volver a intentarlo" }));

    expect(onComposeStamp).toHaveBeenCalled();
  });

  /**
   * El hueco del botón queda reservado incluso vacío: dos estados sin botón
   * —congelado y componiendo— pintan igual de elementos, y el que sí tiene
   * botón no añade una fila nueva, solo lo rellena.
   */
  it("keeps the same button slot whether there is a button or not", async () => {
    const { document, renders } = recordingDocument();
    const { container, rerender } = renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={noop}
        onOpen={noop}
        stamp={{ kind: "frozen" }}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));
    const slot = () => container.querySelector(".viewer__stamp-slot") as HTMLElement;
    expect(slot()).toBeInTheDocument();
    expect(within(slot()).queryByRole("button")).not.toBeInTheDocument();

    rerender(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={noop}
        onOpen={noop}
        stamp={{ kind: "onDemand" }}
      />,
    );

    expect(slot()).toBeInTheDocument();
    expect(within(slot()).getByRole("button", { name: "Ver cómo queda" })).toBeInTheDocument();
  });

  /**
   * El criterio que da nombre al #202: la pastilla sigue visible con el
   * documento ampliado. jsdom no lee el `position: absolute` de la hoja, pero
   * sí el sitio en el DOM — la pastilla tiene que quedar **fuera** del área de
   * desplazamiento, no dentro, o el zoom volvería a taparla.
   */
  it("floats outside the scroll area, so zooming the sheet does not hide it", async () => {
    const { document, renders } = recordingDocument();
    const { container } = renderWithCatalog(
      <DocumentViewer
        pdf={document}
        placement={seated}
        onPlace={noop}
        onOpen={noop}
        stamp={{ kind: "frozen" }}
      />,
    );
    await waitFor(() => expect(renders).toHaveLength(1));

    const scroll = container.querySelector(".viewer__scroll") as HTMLElement;
    const pill = container.querySelector(".viewer__stamp") as HTMLElement;
    expect(pill).toBeInTheDocument();
    expect(scroll).not.toContainElement(pill);
  });
});

/** ID-96: el mismo recuadro en todas las páginas del conjunto, y en ninguna más. */
