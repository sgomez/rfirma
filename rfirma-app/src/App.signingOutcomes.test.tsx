import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { aCertificate, document, openPdf, pdfsOf, renderApp } from "./App.testSupport";
import { inMemoryRecents } from "./documents/recents";
import type { Certificate } from "./signing/certificate";
import type { SigningBackend } from "./signing/flow";
import { NO_PREVIOUS_SIGNATURES } from "./signing/previousSignatures";
import { emptyRubricPicker } from "./signing/rubric";

const remembered: Certificate = { ...aCertificate, remembered: true };

// El recuadro, en espacio de usuario: aquí solo importa que exista, porque
// `sign()` necesita una colocación para armar la orden (ID-92).
const rect = { x0: 250, y0: 50, x1: 450, y1: 100 };

function documentPlaced(name: string) {
  return document(name, { placement: { rect, pages: "all" } });
}

/** Una promesa que la prueba resuelve a mano, para congelar una etapa en curso. */
function deferred<T>() {
  let resolve: (value: T) => void = () => {};
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

function aSigner(overrides: Partial<SigningBackend> = {}): SigningBackend {
  return {
    presign: async () => ({ ok: true, value: { kind: "typedOnScreen" } }),
    sign: async () => ({ ok: true, value: undefined }),
    postsign: async () => ({
      ok: true,
      value: { name: "factura-firmado.pdf", folder: "Documentos", sizeBytes: 1200 },
    }),
    padesLowerLeft: async (placement) => [placement.rect[0], placement.rect[1]],
    previousSignatures: async () => ({
      signatures: [],
      warningCount: 0,
      tone: "information",
      changedAfterLastSignature: false,
      findings: [],
    }),
    signedDocumentSignatures: async () => NO_PREVIOUS_SIGNATURES,
    discard: async () => {},
    ...overrides,
  };
}

/**
 * Grada A (TD-17): firmando, firmado y error sobre la ventana entera, con el
 * soporte de pruebas existente y dobles en memoria (docs/design/panel-de-firma.md,
 * docs/design/dialogo-progreso-firma.md).
 */
describe("App, firmando, firmado y error", () => {
  it("shows the veil progress dialog, locks the other tabs, and frees them once signed", async () => {
    const user = userEvent.setup();
    const signGate = deferred<{ ok: true; value: undefined }>();
    const signer = aSigner({ sign: () => signGate.promise });
    renderApp({
      recents: inMemoryRecents(),
      documents: [documentPlaced("factura.pdf"), documentPlaced("otro.pdf")],
      pdfs: pdfsOf({ "factura.pdf": 2, "otro.pdf": 3 }),
      settings: {},
      certificates: { list: async () => [remembered] },
      rubrics: emptyRubricPicker(),
      signer,
    });

    await openPdf(user);
    await openPdf(user);
    await user.click(screen.getByRole("tab", { name: "factura.pdf" }));

    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const sign = await within(panel).findByRole("button", { name: "Firmar" });
    await waitFor(() => expect(sign).toBeEnabled());
    await user.click(sign);

    // Firmando: el diálogo con velo, en el mismo sitio que el del PIN.
    const progress = await screen.findByRole("dialog", { name: "Firmando el documento…" });
    expect(progress).toBeVisible();

    // Con una firma en curso, el botón de la otra pestaña está bloqueado.
    const otherTab = screen.getByRole("tab", { name: "otro.pdf" });
    expect(otherTab).toBeDisabled();
    await user.click(otherTab);
    expect(screen.getByRole("tab", { name: "factura.pdf" })).toHaveAttribute(
      "aria-selected",
      "true",
    );

    signGate.resolve({ ok: true, value: undefined });

    // Firmado: el resumen y el pie con sus tres salidas.
    expect(await screen.findByText("Firmas del documento")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Abrir el PDF" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Abrir la carpeta" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Firmar" })).toBeInTheDocument();
    expect(
      screen.queryByRole("dialog", { name: "Firmando el documento…" }),
    ).not.toBeInTheDocument();

    // Y la otra pestaña vuelve a estar disponible.
    expect(screen.getByRole("tab", { name: "otro.pdf" })).toBeEnabled();
  });
});
