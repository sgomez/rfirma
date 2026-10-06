import { fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { toUserSpace } from "../viewer/signatureBox";
import { box, recordingDocument, sheet, viewportAt } from "../viewer/testing/fixtures";
import { SedeWindow } from "./SedeWindow";
import { scriptedErrand } from "./testing/fixtures/sedeWindow";

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

  it("hands on the traced box on every page when the person chooses all of them", async () => {
    const { document, renders } = recordingDocument();
    const { port, calls } = scriptedErrand({ kind: "marking", pdf: document });
    renderWithCatalog(<SedeWindow errands={port} />);
    await waitFor(() => expect(renders).toHaveLength(1));

    traceOver(sheet(), [100, 100], [300, 200]);
    await userEvent.click(screen.getByRole("radio", { name: "Todas" }));
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }));

    await waitFor(() =>
      expect(calls.markArea).toHaveBeenCalledWith(expect.objectContaining({ pages: "all" })),
    );
  });

  it("keeps going on hold while the typed pages make no sense", async () => {
    const { document, renders } = recordingDocument();
    const { port } = scriptedErrand({ kind: "marking", pdf: document });
    renderWithCatalog(<SedeWindow errands={port} />);
    await waitFor(() => expect(renders).toHaveLength(1));

    traceOver(sheet(), [100, 100], [300, 200]);
    await userEvent.click(screen.getByRole("radio", { name: "Varias" }));
    await userEvent.clear(screen.getByRole("textbox", { name: "Páginas de la firma visible" }));

    expect(screen.getByRole("button", { name: "Continuar" })).toBeDisabled();
  });

  it("moves the box with the arrows and hands it on with Enter, with the focus on the box", async () => {
    const { document, renders } = recordingDocument();
    const { port, calls } = scriptedErrand({ kind: "marking", pdf: document });
    renderWithCatalog(<SedeWindow errands={port} />);
    await waitFor(() => expect(renders).toHaveLength(1));

    traceOver(sheet(), [100, 100], [300, 200]);
    box().focus();
    await userEvent.keyboard("{ArrowRight}{Enter}");

    const rect = toUserSpace(viewportAt(1), { x: 101, y: 100, width: 200, height: 100 });
    await waitFor(() =>
      expect(calls.markArea).toHaveBeenCalledWith(
        expect.objectContaining({ rect: [rect.x0, rect.y0, rect.x1, rect.y1] }),
      ),
    );
  });

  it("leaves Enter to the pages field, without handing on a half-typed range", async () => {
    const { document, renders } = recordingDocument();
    const { port, calls } = scriptedErrand({ kind: "marking", pdf: document });
    renderWithCatalog(<SedeWindow errands={port} />);
    await waitFor(() => expect(renders).toHaveLength(1));

    traceOver(sheet(), [100, 100], [300, 200]);
    await userEvent.click(screen.getByRole("radio", { name: "Varias" }));
    const pages = screen.getByRole("textbox", { name: "Páginas de la firma visible" });
    await userEvent.clear(pages);
    await userEvent.type(pages, "1,2");
    expect(screen.getByRole("button", { name: "Continuar" })).toBeEnabled();
    await userEvent.type(pages, "{Enter}");

    expect(calls.markArea).not.toHaveBeenCalled();
  });

  it("does nothing on Enter while there is no area", async () => {
    const { document, renders } = recordingDocument();
    const { port, calls } = scriptedErrand({ kind: "marking", pdf: document });
    renderWithCatalog(<SedeWindow errands={port} />);
    await waitFor(() => expect(renders).toHaveLength(1));

    sheet().focus();
    await userEvent.keyboard("{Enter}");

    expect(calls.markArea).not.toHaveBeenCalled();
  });

  it("does nothing on Enter while the typed pages make no sense", async () => {
    const { document, renders } = recordingDocument();
    const { port, calls } = scriptedErrand({ kind: "marking", pdf: document });
    renderWithCatalog(<SedeWindow errands={port} />);
    await waitFor(() => expect(renders).toHaveLength(1));

    traceOver(sheet(), [100, 100], [300, 200]);
    await userEvent.click(screen.getByRole("radio", { name: "Varias" }));
    await userEvent.clear(screen.getByRole("textbox", { name: "Páginas de la firma visible" }));
    box().focus();
    await userEvent.keyboard("{Enter}");

    expect(calls.markArea).not.toHaveBeenCalled();
  });

  it("cancels on Escape with the focus on the pages field", async () => {
    const { document, renders } = recordingDocument();
    const { port, calls } = scriptedErrand({ kind: "marking", pdf: document });
    renderWithCatalog(<SedeWindow errands={port} />);
    await waitFor(() => expect(renders).toHaveLength(1));

    traceOver(sheet(), [100, 100], [300, 200]);
    await userEvent.click(screen.getByRole("radio", { name: "Varias" }));
    await userEvent.type(
      screen.getByRole("textbox", { name: "Páginas de la firma visible" }),
      "{Escape}",
    );

    expect(calls.markArea).toHaveBeenCalledWith(null);
  });

  it("cancels the dialog of the area, not the errand", async () => {
    const { document } = recordingDocument();
    const { port, calls } = scriptedErrand({ kind: "marking", pdf: document });
    renderWithCatalog(<SedeWindow errands={port} />);

    await userEvent.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(calls.markArea).toHaveBeenCalledWith(null);
    expect(calls.cancel).not.toHaveBeenCalled();
  });
});
