import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { aCertificate, document, pdfsOf, renderApp, row } from "./App.testSupport";
import { inMemoryDocumentDrops } from "./documents/drops";
import { inMemoryRecents } from "./documents/recents";
import type { SignedDocumentOpener } from "./signing/destination";
import type { SigningBackend } from "./signing/flow";
import { NO_PREVIOUS_SIGNATURES } from "./signing/previousSignatures";
import { emptyRubricPicker } from "./signing/rubric";

const aSignature = (name: string) => ({
  name,
  idNumber: "00000000T",
  organizationIdentifier: null,
  issuer: "AC FNMT Usuarios",
  certificateSerialNumber: "1",
  signingTime: "2026-09-14T10:32:05Z",
  status: "valid" as const,
  validity: "valid" as const,
  validityReason: null,
  signingDate: null,
  closesDocument: false,
  reason: null,
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
    unregisteredSignatures: async () => false,
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
    return renderApp(
      inMemoryRecents([row("factura.pdf", { folder: "Contratos" })]),
      [],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      { list: async () => [{ ...aCertificate, remembered: true }] },
      emptyRubricPicker(),
      signer,
      invokedToSee("factura.pdf"),
      inMemoryDocumentDrops(invokedToSee("factura.pdf")),
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      null,
      opener,
    );
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

  it("says «Sin firmas» when the PDF has none", async () => {
    render(aSigner());

    expect(await screen.findByText("Sin firmas")).toBeInTheDocument();
    expect(screen.getByText("El documento no tiene firmas.")).toBeInTheDocument();
    expect(screen.queryByRole("listitem")).not.toBeInTheDocument();
  });

  it("shows the error box with its detail and no list when the signatures cannot be read", async () => {
    render(
      aSigner({
        previousSignatures: () =>
          Promise.reject({ situation: "bridgeFailed", detail: "SAF_99: el puente no responde" }),
      }),
    );

    expect(await screen.findByText("No se han podido leer las firmas")).toBeInTheDocument();
    expect(screen.getByText(/SAF_99: el puente no responde/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Copiar detalle" })).toBeInTheDocument();
    expect(screen.queryByRole("listitem")).not.toBeInTheDocument();
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
      return renderApp(
        inMemoryRecents([row("datos.csig", { folder: "Contratos" })]),
        [],
        pdfsOf({}),
        {},
        { list: async () => [{ ...aCertificate, remembered: true }] },
        emptyRubricPicker(),
        signer,
        invokedToSee("datos.csig"),
        inMemoryDocumentDrops(invokedToSee("datos.csig")),
      );
    }

    const cadesWith =
      (...names: string[]) =>
      async () => ({
        ...(await withSignatures(...names)()),
        format: "cades" as const,
      });

    it("shows the signatures of a CAdES with its format, and the viewer says there is no preview", async () => {
      renderNotAPdf(aSigner({ previousSignatures: cadesWith("GRACE HOPPER", "ADA LOVELACE") }));

      expect(await screen.findByText("2 firmas")).toBeInTheDocument();
      expect(screen.getByText("CAdES")).toBeInTheDocument();
      expect(screen.queryByText("PAdES")).not.toBeInTheDocument();
      expect(screen.getByText("GRACE HOPPER (00000000T)")).toBeInTheDocument();
      expect(screen.getByText("Sin vista previa")).toBeInTheDocument();
      expect(screen.queryByText("No se ha podido leer el documento")).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /zoom/i })).not.toBeInTheDocument();
      expect(globalThis.document.querySelector(".viewer__bar")).toBeNull();
    });

    it("nests each countersignature inside its signature, at any depth, and counts both", async () => {
      const deep = { ...aSignature("DEEP SIGNER"), countersignatures: [] };
      const child = { ...aSignature("CHILD SIGNER"), countersignatures: [deep] };
      const sibling = aSignature("SIBLING SIGNER");
      const first = { ...aSignature("FIRST SIGNER"), countersignatures: [child, sibling] };
      renderNotAPdf(
        aSigner({
          previousSignatures: async () => ({
            ...(await withSignatures()()),
            signatures: [first, aSignature("SECOND SIGNER")],
            format: "cades" as const,
          }),
        }),
      );

      expect(await screen.findByText("2 firmas · 3 contrafirmas")).toBeInTheDocument();
      const firstCard = screen.getByText("Firma 1").closest("li") as HTMLElement;
      expect(within(firstCard).getByText("FIRST SIGNER (00000000T)")).toBeInTheDocument();
      expect(within(firstCard).getByText("Contrafirma 1.1")).toBeInTheDocument();
      expect(within(firstCard).getByText("Contrafirma 1.1.1")).toBeInTheDocument();
      expect(within(firstCard).getByText("Contrafirma 1.2")).toBeInTheDocument();
      expect(within(firstCard).getByText("DEEP SIGNER (00000000T)")).toBeInTheDocument();
      const secondCard = screen.getByText("Firma 2").closest("li") as HTMLElement;
      expect(within(secondCard).queryByText(/Contrafirma/)).not.toBeInTheDocument();
    });

    it("says «1 contrafirma» in the singular", async () => {
      const signed = { ...aSignature("FIRST SIGNER"), countersignatures: [aSignature("OTHER")] };
      renderNotAPdf(
        aSigner({
          previousSignatures: async () => ({
            ...(await withSignatures()()),
            signatures: [signed],
            format: "cades" as const,
          }),
        }),
      );

      expect(await screen.findByText("1 firma · 1 contrafirma")).toBeInTheDocument();
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

    it("says «Formato no reconocido» for a file of an unknown format", async () => {
      renderNotAPdf(
        aSigner({
          previousSignatures: async () => ({
            ...NO_PREVIOUS_SIGNATURES,
            format: "unrecognized" as const,
          }),
        }),
      );

      expect(await screen.findByText("Formato no reconocido")).toBeInTheDocument();
      expect(screen.getByText("No es un PDF ni una firma CAdES o XAdES.")).toBeInTheDocument();
      expect(screen.queryByText("Sin firmas")).not.toBeInTheDocument();
      expect(screen.queryByRole("listitem")).not.toBeInTheDocument();
    });
  });
});
