import { fireEvent, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { elapse } from "../testing/elapse";
import { renderWithCatalog } from "../testing/render";
import type { SiteDocument } from "./errand";
import { SedeWindow } from "./SedeWindow";
import { LOCAL_BATCH_DOCUMENTS, PADES_SIGNATURES } from "./testing/fixtures/received";
import {
  Batch,
  IdentityData,
  MixedLocalBatch,
  TerminalOrder,
} from "./testing/fixtures/sedeErrands";
import {
  certificate,
  consentStage,
  scriptedErrand,
  scriptedFrom,
  signedDocument,
} from "./testing/fixtures/sedeWindow";

/** Grada A: la ampliada del consentimiento, que abre «Ver documentos» y cierra «Ocultar». */

const BAREMO: SiteDocument = {
  ...signedDocument,
  name: "Baremo.xml",
  detected: "xml",
  content: { kind: "xml", text: '<baremo convocatoria="FACHADAS-2026"/>' },
};

function consenting(document: SiteDocument = BAREMO) {
  return scriptedErrand(consentStage({ document }));
}

function depthOf(line: HTMLElement) {
  return line.style.getPropertyValue("--xml-depth");
}

function invoiceFrame() {
  const frame = document.querySelector("iframe");
  if (frame === null) throw new Error("falta el marco de la factura");
  return frame;
}

function footerButtons() {
  const footer = document.querySelector("footer");
  if (footer === null) throw new Error("falta el pie");
  return within(footer).getAllByRole("button");
}

describe("2 · the expanded consent", () => {
  it("puts «Ver documento» first in the footer and asks the window to grow", async () => {
    const user = userEvent.setup();
    const { port, calls } = consenting();
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

    const view = screen.getByRole("button", { name: "Ver documento" });
    expect(footerButtons()[0]).toBe(view);
    await user.click(view);

    expect(calls.expandWindow).toHaveBeenCalledWith(true);
    expect(footerButtons()[0]).toBe(screen.getByRole("button", { name: "Ocultar" }));
    expect(screen.getByText('<baremo convocatoria="FACHADAS-2026"/>')).toBeInTheDocument();
    expect(
      screen.getByRole("navigation", { name: "Vas a firmar 1 documento" }),
    ).toBeInTheDocument();
  });

  it("goes back to the compact with «Ocultar», in the same place, and asks the window to shrink", async () => {
    const user = userEvent.setup();
    const { port, calls } = consenting();
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

    await user.click(screen.getByRole("button", { name: "Ver documento" }));
    await user.click(screen.getByRole("button", { name: "Ocultar" }));

    expect(calls.expandWindow).toHaveBeenLastCalledWith(false);
    expect(footerButtons()[0]).toBe(screen.getByRole("button", { name: "Ver documento" }));
    expect(screen.queryByRole("navigation")).not.toBeInTheDocument();
  });

  it("keeps the chosen certificate across both forms", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedErrand(
      consentStage({
        document: BAREMO,
        certificates: [
          certificate({ id: "first", holderName: "ANA" }),
          certificate({ id: "second", holderName: "ZOE" }),
        ],
      }),
    );
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

    await user.click(screen.getByRole("combobox", { name: "Certificado" }));
    await user.click(screen.getByRole("option", { name: /ZOE/ }));
    await user.click(screen.getByRole("button", { name: "Ver documento" }));
    await user.click(screen.getByRole("button", { name: "Firmar" }));

    expect(calls.consent).toHaveBeenCalledWith("second");
  });

  it("gives the window back its size when consenting from the expanded form", async () => {
    const user = userEvent.setup();
    const { port, calls } = consenting();
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

    await user.click(screen.getByRole("button", { name: "Ver documento" }));
    await user.click(screen.getByRole("button", { name: "Firmar" }));

    expect(calls.consent).toHaveBeenCalledWith("handle-1");
    expect(calls.expandWindow).toHaveBeenLastCalledWith(false);
  });

  it("does not ask the window to shrink when it was never expanded", async () => {
    const user = userEvent.setup();
    const { port, calls } = consenting();
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

    await user.click(screen.getByRole("button", { name: "Firmar" }));

    expect(calls.expandWindow).not.toHaveBeenCalled();
  });

  it.each([
    ["a remote batch", Batch],
    ["a request for your details", IdentityData],
    ["a terminal order", TerminalOrder],
  ])("offers no «Ver documentos» for %s", (_, story) => {
    const { port } = scriptedFrom(story);
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

    expect(screen.queryByRole("button", { name: /^Ver documento/ })).not.toBeInTheDocument();
  });

  describe("a local batch", () => {
    const items = LOCAL_BATCH_DOCUMENTS.slice(0, 3);

    it("lists every element and shows the one picked", async () => {
      const user = userEvent.setup();
      const { port } = scriptedErrand(
        consentStage({ document: null, signing: null, signs: items.length, items }),
      );
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

      await user.click(screen.getByRole("button", { name: "Ver documentos" }));

      const list = screen.getByRole("navigation", { name: "Vas a firmar 3 documentos" });
      expect(within(list).getAllByRole("listitem")).toHaveLength(3);
      await user.click(within(list).getByRole("button", { name: /003/ }));
      expect(screen.getByText(/<baremo/)).toBeInTheDocument();
    });
  });

  describe("a local batch with a precalculated hash among its elements", () => {
    const [first, second] = LOCAL_BATCH_DOCUMENTS;
    if (first === undefined || second === undefined) throw new Error("faltan documentos");
    const items = [first, { ...second, precalculatedHash: true }];

    it("shows it as a hash, with no actions, and leaves the others as they were", async () => {
      const user = userEvent.setup();
      const { port } = scriptedErrand(
        consentStage({ document: null, signing: null, signs: items.length, items }),
      );
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

      await user.click(screen.getByRole("button", { name: "Ver documentos" }));
      const list = screen.getByRole("navigation", { name: "Vas a firmar 2 documentos" });

      expect(within(list).getAllByRole("button", { name: /^Acciones de/ })).toHaveLength(1);
      await user.click(within(list).getByRole("button", { name: "Una huella del documento" }));
      expect(within(list).queryByRole("button", { name: /^Acciones de/ })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Abrir con…" })).not.toBeInTheDocument();
    });
  });

  describe("a mixed local batch", () => {
    it("names a hash as such in the viewer, without a preview or a way out", async () => {
      const user = userEvent.setup();
      const { port } = scriptedFrom(MixedLocalBatch);
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

      await user.click(screen.getByRole("button", { name: "Ver documentos" }));
      const list = screen.getByRole("navigation", { name: "Vas a firmar 4 documentos" });
      const firstDigest = within(list)
        .getAllByRole("button", { name: "Una huella del documento" })
        .at(0);
      if (firstDigest === undefined) throw new Error("esperaba una huella en la lista");
      await user.click(firstDigest);

      expect(
        screen.getByText("Este tipo de archivo no se puede mostrar aquí."),
      ).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Abrir con…" })).not.toBeInTheDocument();
      expect(within(list).queryByRole("button", { name: /^Acciones de/ })).not.toBeInTheDocument();
    });

    it("counts in the download only the documents that are not a hash", async () => {
      const user = userEvent.setup();
      const { port } = scriptedFrom(MixedLocalBatch);
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);

      await user.click(screen.getByRole("button", { name: "Ver documentos" }));

      expect(screen.getByRole("button", { name: "Descargar todo" })).toHaveAttribute(
        "title",
        "Los 2 documentos y sus huellas SHA-256, en un .zip",
      );
    });
  });

  describe("the viewer", () => {
    async function expandWith(document: SiteDocument) {
      const user = userEvent.setup();
      const { port } = consenting(document);
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);
      await user.click(screen.getByRole("button", { name: "Ver documento" }));
    }

    it("shows an image on its sheet, named after the document", async () => {
      await expandWith({
        ...signedDocument,
        name: "Cartel.png",
        detected: "png",
        content: { kind: "image", mediaType: "image/png", bytes: new Uint8Array([1, 2, 3]) },
      });

      expect(screen.getByRole("img", { name: "Cartel.png" })).toHaveAttribute(
        "src",
        "data:image/png;base64,AQID",
      );
    });

    it("shows a one-line XML indented, one element per line", async () => {
      await expandWith({
        ...BAREMO,
        content: {
          kind: "xml",
          text: '<baremo convocatoria="FACHADAS-2026"><criterio id="1"><peso>40</peso></criterio></baremo>',
        },
      });

      const root = screen.getByText('<baremo convocatoria="FACHADAS-2026">');
      expect(depthOf(root)).toBe("0");
      expect(depthOf(screen.getByText('<criterio id="1">'))).toBe("1");
      expect(depthOf(screen.getByText("<peso>40</peso>"))).toBe("2");
      expect(depthOf(screen.getByText("</criterio>"))).toBe("1");
      expect(depthOf(screen.getByText("</baremo>"))).toBe("0");
    });

    it("shows an XML that cannot be parsed as it comes", async () => {
      await expandWith({
        ...BAREMO,
        content: { kind: "xml", text: "<baremo><criterio></baremo>" },
      });

      expect(screen.getByText("<baremo><criterio></baremo>")).toBeInTheDocument();
    });

    const INVOICE: SiteDocument = {
      ...signedDocument,
      name: "Factura 2026-0087.xml",
      detected: "invoice",
      content: { kind: "invoice", src: "rfirma-invoice://localhost/handle-1" },
    };

    it("paints an invoice with a view in a frame that only runs its scripts, named after the document", async () => {
      await expandWith(INVOICE);

      const frame = invoiceFrame();
      expect(frame).toHaveAttribute("title", "Factura 2026-0087.xml");
      expect(frame).toHaveAttribute("sandbox", "allow-scripts");
      expect(frame).toHaveAttribute("src", "rfirma-invoice://localhost/handle-1");
    });

    it("enters the invoice frame and leaves it with the tab key", async () => {
      await expandWith(INVOICE);
      const frame = invoiceFrame();
      // Un `iframe` está en el orden del tabulador sin `tabindex`, y user-event no lo modela.
      expect(frame).not.toHaveAttribute("tabindex");
      frame.focus();
      expect(document.activeElement).toBe(frame);

      expect(fireEvent.keyDown(frame, { key: "Tab" })).toBe(true);
      expect(fireEvent.keyDown(frame, { key: "Tab", shiftKey: true })).toBe(true);
    });

    it("opens and closes the signatures panel over an invoice, and Escape still closes it first", async () => {
      const user = userEvent.setup();
      const { port, calls } = consenting({ ...INVOICE, previousSignatures: PADES_SIGNATURES });
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);
      await user.click(screen.getByRole("button", { name: "Ver documento" }));
      await user.click(screen.getByRole("button", { name: "Ver firmas" }));

      expect(screen.getByRole("dialog", { name: "Firmas del documento" })).toBeInTheDocument();
      fireEvent.keyDown(document, { key: "Escape" });

      expect(screen.queryByRole("dialog", { name: "Firmas del documento" })).toBeNull();
      expect(invoiceFrame()).toBeInTheDocument();
      expect(calls.cancel).not.toHaveBeenCalled();
      fireEvent.keyDown(document, { key: "Escape" });
      expect(calls.cancel).toHaveBeenCalledOnce();
    });

    it("shows the text of a document that is an identifier, as it comes", async () => {
      await expandWith({
        ...signedDocument,
        name: null,
        detected: "text",
        content: { kind: "text", text: "EXP-2026-00451" },
      });

      expect(screen.getByText("EXP-2026-00451")).toBeInTheDocument();
    });

    it("says a document without preview cannot be shown, with its name", async () => {
      await expandWith({ ...signedDocument, name: "Beneficiarios.xlsx", detected: "xlsx" });

      const sheet = screen.getByRole("region", { name: "Beneficiarios.xlsx" });
      expect(sheet).toHaveTextContent("Este tipo de archivo no se puede mostrar aquí.");
    });

    it("marks the row with the icon and the seal of the document", async () => {
      await expandWith({ ...BAREMO, previousSignatures: PADES_SIGNATURES });

      expect(screen.getByRole("button", { current: true })).toHaveAccessibleDescription(
        "Con firmas con incidencias",
      );
    });

    it("closes the signatures panel with Escape without cancelling the errand", async () => {
      const user = userEvent.setup();
      const { port, calls } = consenting({ ...BAREMO, previousSignatures: PADES_SIGNATURES });
      renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);
      await user.click(screen.getByRole("button", { name: "Ver documento" }));
      await user.click(screen.getByRole("button", { name: "Ver firmas" }));

      fireEvent.keyDown(document, { key: "Escape" });

      expect(screen.queryByRole("dialog", { name: "Firmas del documento" })).toBeNull();
      expect(calls.cancel).not.toHaveBeenCalled();
      fireEvent.keyDown(document, { key: "Escape" });
      expect(calls.cancel).toHaveBeenCalledOnce();
    });
  });

  describe("the countdown", () => {
    beforeEach(() => vi.useFakeTimers());
    afterEach(() => vi.useRealTimers());

    it("goes on counting when expanding and hiding", async () => {
      const { port } = consenting();
      renderWithCatalog(<SedeWindow errands={port} />);

      await elapse(1000);
      fireEvent.click(screen.getByRole("button", { name: "Ver documento" }));
      expect(screen.getByRole("button", { name: "Firmar (2)" })).toBeDisabled();

      await elapse(1000);
      fireEvent.click(screen.getByRole("button", { name: "Ocultar" }));
      expect(screen.getByRole("button", { name: "Firmar (1)" })).toBeDisabled();

      await elapse(1000);
      expect(screen.getByRole("button", { name: "Firmar" })).toBeEnabled();
    });
  });
});
