import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { inMemoryDocumentDrops } from "./documents/drops";
import { inMemoryRecents } from "./documents/recents";
import type { SignedDocumentOpener } from "./signing/destination";
import type { SigningBackend } from "./signing/flow";
import { NO_PREVIOUS_SIGNATURES } from "./signing/previousSignatures";
import { emptyRubricPicker } from "./signing/rubric";
import { aCertificate, document, pdfsOf, renderApp, row } from "./testing/harness";

const aSignature = (name: string) => ({
  name,
  idNumber: "00000000T",
  organizationIdentifier: null,
  issuer: "AC FNMT Usuarios",
  certificateSerialNumber: "1",
  signingTime: "2026-09-14T10:32:05Z",
  validity: "valid" as const,
  validityReason: null,
  signingDate: null,
  closesDocument: false,
  countersignatures: [],
});

function aSigner(overrides: Partial<SigningBackend> = {}): SigningBackend {
  return {
    presign: async () => ({ ok: true, value: { kind: "typedOnScreen" } }),
    sign: async () => ({ ok: true, value: undefined }),
    postsign: async () => ({
      ok: true,
      value: { name: "factura-firmado.pdf", folder: "Documentos", sizeBytes: 1200 },
    }),
    padesLowerLeft: async (placement) => [placement.rect[0], placement.rect[1]],
    previousSignatures: async () => NO_PREVIOUS_SIGNATURES,
    signedDocumentSignatures: async () => NO_PREVIOUS_SIGNATURES,
    discard: async () => {},
    ...overrides,
  };
}

const withSignatures =
  (...names: string[]) =>
  async () => ({
    signatures: names.map(aSignature),
    warningCount: 0,
    tone: "information" as const,
    changedAfterLastSignature: false,
    findings: [],
  });

const invokedToSee = (name: string) => ({
  document: document(name),
  alsoEntering: [],
  failure: null,
  discarded: 0,
  seeSignatures: true,
});

/** `verify --gui` sobre la ventana entera: el resumen sin franja ni «Cambiar». */
describe("App, con verify --gui", () => {
  function render(signer: SigningBackend, opener?: SignedDocumentOpener) {
    return renderApp({
      recents: inMemoryRecents([row("factura.pdf", { folder: "Contratos" })]),
      documents: [],
      pdfs: pdfsOf({ "factura.pdf": 2 }),
      settings: {},
      certificates: { list: async () => [{ ...aCertificate, remembered: true }] },
      rubrics: emptyRubricPicker(),
      signer,
      invoked: invokedToSee("factura.pdf"),
      drops: inMemoryDocumentDrops(invokedToSee("factura.pdf")),
      titlebar: null,
      opener,
    });
  }

  it("opens straight onto the summary of the document's signatures, with no strip, badge or Cambiar", async () => {
    render(aSigner({ previousSignatures: withSignatures("GRACE HOPPER", "ADA LOVELACE") }));

    expect(await screen.findByText("2 firmas")).toBeInTheDocument();
    expect(screen.getByText("Firmas del documento")).toBeInTheDocument();
    const [first, second] = screen.getAllByRole("listitem") as [HTMLElement, HTMLElement];
    expect(within(first).getByText("GRACE HOPPER (00000000T)")).toBeInTheDocument();
    expect(within(second).getByText("ADA LOVELACE (00000000T)")).toBeInTheDocument();
    expect(screen.queryByText("Nueva")).not.toBeInTheDocument();
    expect(screen.queryByText(/^Firmado a las /)).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Cambiar" })).not.toBeInTheDocument();
    expect(screen.queryByText("Firmar con", { exact: false })).not.toBeInTheDocument();
    expect(screen.getByText("Documento")).toBeInTheDocument();
    expect(screen.getByText("Contratos")).toBeInTheDocument();
    expect(globalThis.document.querySelector(".panel__destination-name")).toHaveTextContent(
      "factura.pdf",
    );
  });

  it("opens the document and its folder, and Firmar leads to the signing panel for that document", async () => {
    const user = userEvent.setup();
    const opener = {
      openDocument: vi.fn(async () => {}),
      openFolder: vi.fn(async () => {}),
    };
    render(aSigner({ previousSignatures: withSignatures("GRACE HOPPER") }), opener);
    await screen.findByText("1 firma");

    await user.click(screen.getByRole("button", { name: "Abrir el PDF" }));
    await user.click(screen.getByTitle("Abrir la carpeta"));
    expect(opener.openDocument).toHaveBeenCalledWith(expect.any(String));
    expect(opener.openFolder).toHaveBeenCalledWith(expect.any(String));

    await user.click(screen.getByRole("button", { name: "Firmar" }));

    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await waitFor(() =>
      expect(within(panel).getByRole("button", { name: "Firmar" })).toBeEnabled(),
    );
    expect(screen.queryByText("Firmas del documento")).not.toBeInTheDocument();
  });

  describe("with a file that is not a PDF", () => {
    function renderNotAPdf(signer: SigningBackend) {
      return renderApp({
        recents: inMemoryRecents([row("datos.csig", { folder: "Contratos" })]),
        documents: [],
        pdfs: pdfsOf({}),
        settings: {},
        certificates: { list: async () => [{ ...aCertificate, remembered: true }] },
        rubrics: emptyRubricPicker(),
        signer,
        invoked: invokedToSee("datos.csig"),
        drops: inMemoryDocumentDrops(invokedToSee("datos.csig")),
      });
    }

    const cadesWith =
      (...names: string[]) =>
      async () => ({
        ...(await withSignatures(...names)()),
        format: "cades" as const,
      });

    it("mounts the reading beside a viewer that says there is no preview", async () => {
      renderNotAPdf(aSigner({ previousSignatures: cadesWith("GRACE HOPPER", "ADA LOVELACE") }));

      expect(await screen.findByText("2 firmas")).toBeInTheDocument();
      expect(screen.getByText("Sin vista previa")).toBeInTheDocument();
      expect(screen.queryByText("No se ha podido leer el documento")).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /zoom/i })).not.toBeInTheDocument();
      expect(globalThis.document.querySelector(".viewer__bar")).toBeNull();
    });

    it("offers to open the file and disables Firmar with its reason", async () => {
      renderNotAPdf(aSigner({ previousSignatures: cadesWith("GRACE HOPPER") }));
      await screen.findByText("1 firma");

      expect(screen.getByRole("button", { name: "Abrir el fichero" })).toBeEnabled();
      expect(screen.queryByRole("button", { name: "Abrir el PDF" })).not.toBeInTheDocument();
      const sign = screen.getByRole("button", { name: "Firmar" });
      expect(sign).toBeDisabled();
      expect(sign).toHaveAttribute("title", "En el escritorio solo se firman PDF");
    });
  });
});
