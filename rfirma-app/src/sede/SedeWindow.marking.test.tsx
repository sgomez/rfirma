import { fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { toUserSpace } from "../viewer/signatureBox";
import { recordingDocument, sheet, viewportAt } from "../viewer/testing/documentViewerFixtures";
import { SedeWindow } from "./SedeWindow";
import { scriptedErrand } from "./sedeWindowFixtures";

/** Grada A: el momento 1c, el área de la firma visible marcada sobre el PDF. */

function traceOver(element: HTMLElement, from: [number, number], to: [number, number]) {
  fireEvent.pointerDown(element, { pointerId: 1, button: 0, clientX: from[0], clientY: from[1] });
  fireEvent.pointerMove(element, { pointerId: 1, clientX: to[0], clientY: to[1] });
  fireEvent.pointerUp(element, { pointerId: 1, clientX: to[0], clientY: to[1] });
}

describe("1c · marking the area of the visible signature", () => {
  it("asks to mark the area on the document and keeps going on hold until there is one", async () => {
    const { document, renders } = recordingDocument();
    const { port } = scriptedErrand({ kind: "marking", pdf: document });
    renderWithCatalog(<SedeWindow errands={port} />);

    await waitFor(() => expect(renders).toHaveLength(1));
    expect(screen.getByText("Marca dónde va tu firma")).toBeInTheDocument();
    expect(sheet()).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Continuar" })).toBeDisabled();
  });

  it("hands on the traced box with the page it lies on", async () => {
    const { document, renders } = recordingDocument();
    const { port, calls } = scriptedErrand({ kind: "marking", pdf: document });
    renderWithCatalog(<SedeWindow errands={port} />);
    await waitFor(() => expect(renders).toHaveLength(1));

    traceOver(sheet(), [100, 100], [300, 200]);
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }));

    const rect = toUserSpace(viewportAt(1), { x: 100, y: 100, width: 200, height: 100 });
    await waitFor(() =>
      expect(calls.markArea).toHaveBeenCalledWith({
        page: 1,
        pages: { only: [1] },
        pageCount: 3,
        mediaBox: [0, 0, 595, 842],
        rotation: 0,
        rect: [rect.x0, rect.y0, rect.x1, rect.y1],
      }),
    );
  });

  it("cancels the dialog of the area, not the errand", async () => {
    const { document } = recordingDocument();
    const { port, calls } = scriptedErrand({ kind: "marking", pdf: document });
    renderWithCatalog(<SedeWindow errands={port} />);

    await userEvent.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(calls.markArea).toHaveBeenCalledWith(null);
    expect(calls.cancel).not.toHaveBeenCalled();
  });

  it("leaves only cancelling when the document cannot be opened", () => {
    const { port } = scriptedErrand({ kind: "marking", pdf: null });
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByText(/No se ha podido abrir el documento/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Continuar" })).toBeDisabled();
  });
});
