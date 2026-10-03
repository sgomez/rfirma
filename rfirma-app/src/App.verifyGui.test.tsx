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
  reason: null,
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
});
