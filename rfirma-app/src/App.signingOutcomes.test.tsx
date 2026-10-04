import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { aCertificate, document, openPdf, pdfsOf, renderApp } from "./App.testSupport";
import { inMemoryRecents } from "./documents/recents";
import type { Certificate } from "./signing/certificate";
import type { SigningBackend } from "./signing/flow";
import { NO_PREVIOUS_SIGNATURES } from "./signing/previousSignatures";
import { emptyRubricPicker } from "./signing/rubric";
import type { TokenFailure } from "./signing/token";

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
    renderApp(
      inMemoryRecents(),
      [documentPlaced("factura.pdf"), documentPlaced("otro.pdf")],
      pdfsOf({ "factura.pdf": 2, "otro.pdf": 3 }),
      {},
      { list: async () => [remembered] },
      emptyRubricPicker(),
      signer,
    );

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

  it("lists every signature of the signed document after signing, the user's own marked as new", async () => {
    const user = userEvent.setup();
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
    const signer = aSigner({
      signedDocumentSignatures: async () => ({
        signatures: [aSignature("GRACE HOPPER"), aSignature("ADA LOVELACE")],
        warningCount: 0,
        tone: "information",
        changedAfterLastSignature: false,
        findings: [],
      }),
    });
    renderApp(
      inMemoryRecents(),
      [documentPlaced("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      { list: async () => [remembered] },
      emptyRubricPicker(),
      signer,
    );

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const sign = await within(panel).findByRole("button", { name: "Firmar" });
    await waitFor(() => expect(sign).toBeEnabled());
    await user.click(sign);

    expect(await screen.findByText("2 firmas")).toBeInTheDocument();
    const cards = screen.getAllByRole("listitem");
    const [first, second] = cards as [HTMLElement, HTMLElement];
    expect(within(first).getByText("GRACE HOPPER (00000000T)")).toBeInTheDocument();
    expect(within(second).getByText("ADA LOVELACE (00000000T)")).toBeInTheDocument();
    expect(within(second).getByText("Nueva")).toBeInTheDocument();
    expect(within(first).queryByText("Nueva")).not.toBeInTheDocument();
    expect(screen.getByText(/^Firmado a las /)).toBeInTheDocument();
    expect(screen.getByText("Documento")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cambiar" })).toBeVisible();
  });

  it("says what happened, that the document is unchanged, and offers Reintentar and Volver", async () => {
    const user = userEvent.setup();
    const signer = aSigner({
      sign: async () => ({
        ok: false,
        failure: {
          situation: "tokenAbsent",
          detail: "CKR_DEVICE_REMOVED (C_Sign)",
          attemptsLeft: null,
        },
      }),
    });
    renderApp(
      inMemoryRecents(),
      [documentPlaced("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      { list: async () => [remembered] },
      emptyRubricPicker(),
      signer,
    );

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const sign = await within(panel).findByRole("button", { name: "Firmar" });
    await waitFor(() => expect(sign).toBeEnabled());
    await user.click(sign);

    expect(await screen.findByText("Falta la tarjeta o el certificado")).toBeInTheDocument();
    expect(
      screen.getByText("El documento sigue como estaba: no se ha guardado nada."),
    ).toBeInTheDocument();
    // El pie sigue enseñando el destino: solo cambia el botón de firmar.
    expect(screen.getByText("contrato-firmado.pdf")).toBeInTheDocument();

    const retry = screen.getByRole("button", { name: "Reintentar" });
    const back = screen.getByRole("button", { name: "Volver" });
    expect(retry).toBeInTheDocument();

    await user.click(back);

    // «Volver» cierra el error y deja el panel como si nada hubiera pasado.
    expect(screen.queryByText("Falta la tarjeta o el certificado")).not.toBeInTheDocument();
    expect(await within(panel).findByText("Firma visible")).toBeInTheDocument();
  });

  /** Criterio 3 del #1062: firmar con un certificado instalado ofrece vaciar el almacén. */
  it("offers to empty the store when signing fails with a lost keyring pin", async () => {
    const user = userEvent.setup();
    const signer = aSigner({
      // Como en `tauriStage.ts`: `TokenSituation` no cierra sobre las siete de pkcs11.
      sign: async () => ({
        ok: false,
        failure: {
          situation: "keyringPinMissing" as TokenFailure["situation"],
          detail: "PK11_CheckUserPassword: el pin del llavero no abre el almacen ya existente",
          attemptsLeft: null,
        },
      }),
    });
    const emptyStore = vi.fn(async () => {});
    renderApp(
      inMemoryRecents(),
      [documentPlaced("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      { list: async () => [remembered], emptyStore },
      emptyRubricPicker(),
      signer,
    );

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const sign = await within(panel).findByRole("button", { name: "Firmar" });
    await waitFor(() => expect(sign).toBeEnabled());
    await user.click(sign);

    await user.click(await screen.findByRole("button", { name: "Vaciar el almacén" }));
    await user.click(screen.getByRole("button", { name: "Vaciar el almacén" }));

    expect(emptyStore).toHaveBeenCalledOnce();
  });

  it("abandons a failure left on another tab instead of showing it there", async () => {
    const user = userEvent.setup();
    const discard = vi.fn(async () => {});
    const signer = aSigner({
      sign: async () => ({
        ok: false,
        failure: {
          situation: "tokenAbsent",
          detail: "CKR_DEVICE_REMOVED (C_Sign)",
          attemptsLeft: null,
        },
      }),
      discard,
    });
    renderApp(
      inMemoryRecents(),
      [documentPlaced("factura.pdf"), documentPlaced("otro.pdf")],
      pdfsOf({ "factura.pdf": 2, "otro.pdf": 3 }),
      {},
      { list: async () => [remembered] },
      emptyRubricPicker(),
      signer,
    );

    await openPdf(user);
    await openPdf(user);
    await user.click(screen.getByRole("tab", { name: "factura.pdf" }));
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const sign = await within(panel).findByRole("button", { name: "Firmar" });
    await waitFor(() => expect(sign).toBeEnabled());
    await user.click(sign);
    await screen.findByText("Falta la tarjeta o el certificado");

    await user.click(screen.getByRole("tab", { name: "otro.pdf" }));

    // El ciclo a medias se olvida en el backend, como pulsar «Volver» a mano.
    await waitFor(() => expect(discard).toHaveBeenCalled());
    expect(screen.queryByText("Falta la tarjeta o el certificado")).not.toBeInTheDocument();
  });
});
