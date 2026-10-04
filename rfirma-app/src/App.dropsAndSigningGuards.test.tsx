import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import {
  aCertificate,
  document,
  openPdf,
  pdfsOf,
  pdfsWithViews,
  renderApp,
} from "./App.testSupport";
import type { DocumentInHand } from "./documents/document";
import { inMemoryRecents } from "./documents/recents";
import type { Certificate } from "./signing/certificate";
import type { SigningBackend, SigningOrder } from "./signing/flow";
import type { PreviousSignature, PreviousSignaturesReport } from "./signing/previousSignatures";
import { NO_PREVIOUS_SIGNATURES } from "./signing/previousSignatures";
import { emptyRubricPicker } from "./signing/rubric";
import type { Placement } from "./viewer/signatureBox";

/**
 * **Grada A del arrastre** (TD-17): los cuatro casos, contados por lo que se ve
 * en la ventana y no por lo que se llamó.
 *
 * Se prueban contra el doble del puerto porque en Tauri v2 el arrastre es un
 * evento nativo de la ventana: `jsdom` no lo tiene, y un `fireEvent.drop` sobre
 * el JSX probaría un camino que en la aplicación de verdad **no existe**
 * (ID-67). Lo que el doble entrega es lo mismo que emite Rust, que es quien
 * decide qué se abre de lo soltado.
 */
describe("App, al soltar ficheros en la ventana", () => {
  /** Lo que Rust emite al soltar un PDF que sí se abre. */
  function anOpened(name: string, alsoEntering: DocumentInHand[] = [], discarded = 0) {
    return { document: document(name), alsoEntering, failure: null, discarded };
  }

  it("opens a dropped PDF exactly like the dialog does", async () => {
    const { drops } = renderApp(inMemoryRecents(), [], pdfsOf({ "factura.pdf": 7 }));

    drops.drop(anOpened("factura.pdf"));

    await screen.findByRole("region", { name: "Panel de firma" });
    expect(screen.getByRole("tab", { name: "factura.pdf", selected: true })).toBeInTheDocument();
    expect(screen.getByRole("banner")).not.toHaveTextContent("Sin firmar");
  });

  it("says so when what was dropped is not a PDF", async () => {
    const { drops } = renderApp();

    drops.drop({
      document: null,
      alsoEntering: [],
      failure: { situation: "notAPdf", detail: "el fichero no es un PDF" },
      discarded: 0,
    });

    expect(await screen.findByRole("alert")).toHaveTextContent("Ese fichero no es un PDF");
  });

  it("opens a tab for each of several dropped PDFs, with the first one in front", async () => {
    const { drops } = renderApp(inMemoryRecents(), [], pdfsOf({ "factura.pdf": 2 }));

    drops.drop(anOpened("factura.pdf", [document("contrato.pdf")]));

    await screen.findByRole("region", { name: "Panel de firma" });
    expect(await screen.findByRole("tab", { name: "contrato.pdf", selected: false })).toBeVisible();
    expect(screen.getByRole("tab", { name: "factura.pdf", selected: true })).toBeVisible();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  /**
   * ID-306: soltar N ficheros abre N pestañas, no una, y a la vez
   * se dice cuántos se descartaron — las dos cosas del mismo gesto, no dos
   * casos por separado.
   */
  it("opens N tabs for N dropped files and counts the discarded ones in the same gesture", async () => {
    const { drops } = renderApp(inMemoryRecents(), [], pdfsOf({ "factura.pdf": 2 }));

    drops.drop(anOpened("factura.pdf", [document("contrato.pdf"), document("anexo.pdf")], 2));

    await screen.findByRole("region", { name: "Panel de firma" });
    expect(await screen.findByRole("tab", { name: "contrato.pdf" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "anexo.pdf" })).toBeInTheDocument();
    const notice = await screen.findByRole("alert");
    expect(notice).toHaveTextContent("Algunos ficheros no se han añadido");
    expect(notice).toHaveTextContent("se han descartado 2 ficheros");
  });

  /** ID-306: lo que no era un PDF sí se cuenta, y por qué. */
  it("says how many were discarded when some of what was dropped was not a PDF", async () => {
    const { drops } = renderApp(inMemoryRecents(), [], pdfsOf({ "factura.pdf": 2 }));

    drops.drop(anOpened("factura.pdf", [], 2));

    await screen.findByRole("region", { name: "Panel de firma" });
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Algunos ficheros no se han añadido",
    );
  });

  /**
   * ID-68: el aviso dice **qué hacer**, y lo que hay que hacer es usar el botón
   * de abrir, que sí pasa por el portal. Que este caso exista de verdad —y
   * desde qué carpetas— está medido en
   * `docs/research/arrastre-bajo-el-sandbox.md`.
   */
  it("tells what to do when the dropped file cannot be read", async () => {
    const { drops } = renderApp();

    drops.drop({
      document: null,
      alsoEntering: [],
      failure: {
        situation: "droppedFileUnreadable",
        detail: "No such file or directory (os error 2)",
      },
      discarded: 0,
    });

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("No hemos podido leer el fichero que has soltado");
    expect(alert).toHaveTextContent("Ábrelo con el botón de abrir");
    // Y el detalle crudo sigue ahí, sin traducir, para el informe de fallo.
    expect(alert).toHaveTextContent("os error 2");
  });

  /** El aviso habla del documento que hay delante, así que se va con él. */
  it("drops the notice once another document is in front", async () => {
    const user = userEvent.setup();
    const { drops } = renderApp(
      inMemoryRecents(),
      [document("otro.pdf")],
      pdfsOf({ "factura.pdf": 2, "otro.pdf": 3 }),
    );
    drops.drop(anOpened("factura.pdf", [], 2));
    await screen.findByRole("alert");

    await openPdf(user);

    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  });
});

/**
 * ID-105/ID-106: el diálogo de páginas sin sello, justo antes de firmar y
 * gateado en `App.sign` (docs/design/dialogo-paginas-sin-firma-visible.md).
 */
describe("App, con páginas donde el recuadro no cabe", () => {
  const A4: readonly [number, number, number, number] = [0, 0, 595, 842];
  // Más pequeña que el recuadro que se coloca abajo: se cae.
  const SMALL: readonly [number, number, number, number] = [0, 0, 200, 150];

  const remembered: Certificate = { ...aCertificate, remembered: true };

  // El recuadro cabe en A4 pero no en SMALL: fitsInPage lo comprueba contra
  // el ancho y el alto, igual que correctPositionSignature.
  const rect = { x0: 250, y0: 50, x1: 450, y1: 100 };

  function documentWithPlacement(name: string, pages: Placement["pages"]) {
    return document(name, { placement: { rect, pages } });
  }

  it("warns before signing when some of the chosen pages will fall, and cancel does not sign", async () => {
    const user = userEvent.setup();
    const presign = vi.fn(async () => ({
      ok: true as const,
      value: { kind: "typedOnScreen" as const },
    }));
    const signer: SigningBackend = {
      presign,
      sign: async () => ({ ok: true, value: undefined }),
      postsign: async () => ({
        ok: true,
        value: { name: "factura.pdf", folder: "Documentos", sizeBytes: 1 },
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
    };
    renderApp(
      inMemoryRecents(),
      [documentWithPlacement("factura.pdf", { only: [1, 2, 3] })],
      pdfsWithViews("factura.pdf", [A4, SMALL, A4]),
      {},
      { list: async () => [remembered] },
      emptyRubricPicker(),
      signer,
    );

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const sign = await within(panel).findByRole("button", { name: "Firmar" });
    await waitFor(() => expect(sign).toBeEnabled());
    await user.click(within(panel).getByRole("switch", { name: "Firma visible" }));

    await user.click(sign);

    expect(
      await screen.findByRole("dialog", { name: "Una página se quedará sin firma visible" }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "El recuadro no cabe en páginas más pequeñas que aquella donde lo colocaste. " +
          "La firma será válida en todo el documento.",
      ),
    ).toBeInTheDocument();
    expect(presign).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(presign).not.toHaveBeenCalled();
  });

  it("signs anyway with the exact order already built, when confirmed", async () => {
    const user = userEvent.setup();
    const presign = vi.fn(async (_order: SigningOrder) => ({
      ok: true as const,
      value: { kind: "typedOnScreen" as const },
    }));
    const signer: SigningBackend = {
      presign,
      sign: async () => ({ ok: true, value: undefined }),
      postsign: async () => ({
        ok: true,
        value: { name: "factura.pdf", folder: "Documentos", sizeBytes: 1 },
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
    };
    renderApp(
      inMemoryRecents(),
      [documentWithPlacement("factura.pdf", { only: [1, 2, 3] })],
      pdfsWithViews("factura.pdf", [A4, SMALL, A4]),
      {},
      { list: async () => [remembered] },
      emptyRubricPicker(),
      signer,
    );

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const sign = await within(panel).findByRole("button", { name: "Firmar" });
    await waitFor(() => expect(sign).toBeEnabled());
    await user.click(within(panel).getByRole("switch", { name: "Firma visible" }));
    await user.click(sign);
    await screen.findByRole("dialog", { name: "Una página se quedará sin firma visible" });

    await user.click(screen.getByRole("button", { name: "Firmar de todos modos" }));

    await waitFor(() => expect(presign).toHaveBeenCalledOnce());
    const order = presign.mock.calls[0]?.[0];
    expect(order?.placement?.pages).toEqual({ only: [1, 2, 3] });
    expect(screen.queryByRole("dialog", { name: /firma visible/ })).not.toBeInTheDocument();
  });

  it("does not appear when every chosen page keeps its visible signature", async () => {
    const user = userEvent.setup();
    const presign = vi.fn(async () => ({
      ok: true as const,
      value: { kind: "typedOnScreen" as const },
    }));
    const signer: SigningBackend = {
      presign,
      sign: async () => ({ ok: true, value: undefined }),
      postsign: async () => ({
        ok: true,
        value: { name: "factura.pdf", folder: "Documentos", sizeBytes: 1 },
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
    };
    renderApp(
      inMemoryRecents(),
      [documentWithPlacement("factura.pdf", { only: [1, 3] })],
      pdfsWithViews("factura.pdf", [A4, SMALL, A4]),
      {},
      { list: async () => [remembered] },
      emptyRubricPicker(),
      signer,
    );

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const sign = await within(panel).findByRole("button", { name: "Firmar" });
    await waitFor(() => expect(sign).toBeEnabled());
    await user.click(within(panel).getByRole("switch", { name: "Firma visible" }));

    await user.click(sign);

    await waitFor(() => expect(presign).toHaveBeenCalledOnce());
    expect(screen.queryByRole("dialog", { name: /firma visible/ })).not.toBeInTheDocument();
  });
});

/**
 * **TD-64**: la ventana distingue el documento que se firma de la fila que se
 * guarda (ID-287), y sabe pintar y firmar uno del que no queda rastro (ID-286).
 *
 * Se prueba por los puertos doblados —el selector entrega el documento, la
 * bandeja es el almacén en memoria— porque eso es exactamente lo que hará la
 * sede: entregar un documento por un puerto, sin fila detrás.
 */
describe("App, con un documento que no se recuerda", () => {
  const remembered: Certificate = { ...aCertificate, remembered: true };

  /** Un recuadro ya colocado, para llegar a «Firmar documento» sin gestos. */
  const aPlacement: Placement = {
    rect: { x0: 250, y0: 50, x1: 450, y1: 100 },
    pages: { only: [1] },
  };

  /** Lo que mandará la sede: se pinta y se firma, pero no se guarda. */
  const fromTheSede = () => document("de-la-sede.pdf", { remembered: false });

  it("paints it in the viewer without leaving it among the recents", async () => {
    const user = userEvent.setup();
    const recents = inMemoryRecents();
    renderApp(recents, [fromTheSede()], pdfsOf({ "de-la-sede.pdf": 4 }));

    await openPdf(user);

    await screen.findByRole("region", { name: "Panel de firma" });
    expect(screen.getByRole("tab", { name: "de-la-sede.pdf", selected: true })).toBeInTheDocument();
    await expect(recents.list()).resolves.toEqual([]);
  });

  it("leaves no placement behind when the box is put on it", async () => {
    const user = userEvent.setup();
    const recents = inMemoryRecents();
    renderApp(
      recents,
      [fromTheSede()],
      pdfsOf({ "de-la-sede.pdf": 4 }),
      {},
      { list: async () => [remembered] },
    );
    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await user.click(within(panel).getByRole("switch", { name: "Firma visible" }));
    await within(panel).findByText("En la página 1");

    await user.click(within(panel).getByRole("radio", { name: "Todas" }));

    // El recuadro está puesto —la ventana lo pinta— y aun así no se ha escrito
    // nada: no hay fila donde apuntarlo.
    expect(
      screen.queryByRole("application", { name: "Recuadro de la firma visible" }),
    ).not.toBeNull();
    await expect(recents.list()).resolves.toEqual([]);
  });

  it("signs it, and signing it still leaves no row", async () => {
    const user = userEvent.setup();
    const recents = inMemoryRecents();
    const presign = vi.fn(async () => ({
      ok: true as const,
      value: { kind: "typedOnScreen" as const },
    }));
    const signer: SigningBackend = {
      presign,
      sign: async () => ({ ok: true, value: undefined }),
      postsign: async () => ({
        ok: true,
        value: { name: "de-la-sede-firmado.pdf", folder: "Documentos", sizeBytes: 1 },
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
    };
    renderApp(
      recents,
      [document("de-la-sede.pdf", { remembered: false, placement: aPlacement })],
      pdfsOf({ "de-la-sede.pdf": 4 }),
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

    await waitFor(() => expect(presign).toHaveBeenCalledOnce());
    await expect(recents.list()).resolves.toEqual([]);
  });
});

/**
 * El diálogo «¿Firmar de todos modos?» (docs/design/dialogo-firmar-de-todos-modos.md).
 *
 * Vive en la grada A y aquí, junto a los otros avisos antes del PIN, porque lo
 * que se prueba es la fila completa: el gateo del botón contra el informe de
 * firmas previas, una fila por problema y las dos salidas del diálogo.
 */
describe("App · ¿Firmar de todos modos?", () => {
  const remembered: Certificate = { ...aCertificate, remembered: true };

  const aPlacement: Placement = {
    rect: { x0: 250, y0: 50, x1: 450, y1: 100 },
    pages: { only: [1] },
  };

  function aSignature(overrides: Partial<PreviousSignature>): PreviousSignature {
    return {
      name: "Bruce Wayne",
      idNumber: "00000000T",
      organizationIdentifier: null,
      issuer: "AC FNMT Usuarios",
      certificateSerialNumber: "1",
      signingTime: "2024-01-01T10:00:00.000Z",
      validity: "valid",
      validityReason: null,
      signingDate: null,
      closesDocument: false,
      countersignatures: [],
      ...overrides,
    };
  }

  const validSignature = aSignature({ name: "Alfred Pennyworth" });
  const expiredSignature = aSignature({
    name: "Bruce Wayne",
    validity: "expired",
    validityReason: { kind: "certificateExpired", date: "2020-03-05T12:00:00Z", holder: null },
  });
  const unknownTypeSignature = aSignature({
    name: "Notaría XYZ",
    validity: "invalid",
    validityReason: { kind: "unknownSignatureType" },
  });

  function signerOver(
    report: Partial<PreviousSignaturesReport>,
    presign: SigningBackend["presign"],
  ): SigningBackend {
    return {
      presign,
      sign: async () => ({ ok: true, value: undefined }),
      postsign: async () => ({
        ok: true,
        value: { name: "cofirmado-firmado.pdf", folder: "Documentos", sizeBytes: 1 },
      }),
      padesLowerLeft: async (placement) => [placement.rect[0], placement.rect[1]],
      previousSignatures: async () => ({
        ...NO_PREVIOUS_SIGNATURES,
        tone: "attention",
        ...report,
      }),
      signedDocumentSignatures: async () => NO_PREVIOUS_SIGNATURES,
      discard: async () => {},
    };
  }

  async function readyToSign(signer: SigningBackend) {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("cofirmado.pdf", { placement: aPlacement })],
      pdfsOf({ "cofirmado.pdf": 4 }),
      {},
      { list: async () => [remembered] },
      emptyRubricPicker(),
      signer,
    );
    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const sign = await within(panel).findByRole("button", { name: "Firmar" });
    await waitFor(() => expect(sign).toBeEnabled());
    return { user, sign };
  }

  function recordingPresign(presigned: SigningOrder[]): SigningBackend["presign"] {
    return async (order) => {
      presigned.push(order);
      return { ok: true, value: { kind: "typedOnScreen" } };
    };
  }

  it("opens for an expired signature, with its row and reason, and does not sign", async () => {
    const presigned: SigningOrder[] = [];
    const { user, sign } = await readyToSign(
      signerOver({ signatures: [expiredSignature] }, recordingPresign(presigned)),
    );

    await user.click(sign);

    expect(presigned).toHaveLength(0);
    const dialog = await screen.findByRole("dialog", { name: "¿Firmar de todos modos?" });
    expect(within(dialog).getByText("Firma 1 · Bruce Wayne")).toBeInTheDocument();
    expect(within(dialog).getByText(/^El certificado caducó el .*2020$/)).toBeInTheDocument();
    expect(within(dialog).getByText("El receptor podría rechazarlo.")).toBeInTheDocument();
  });

  it("opens for a finding alone, as a row without a second line", async () => {
    const { user, sign } = await readyToSign(
      signerOver(
        { signatures: [validSignature], findings: ["modifiedAfterLastSignature"] },
        recordingPresign([]),
      ),
    );

    await user.click(sign);

    const dialog = await screen.findByRole("dialog", { name: "¿Firmar de todos modos?" });
    expect(within(dialog).getAllByRole("listitem")).toHaveLength(1);
    expect(
      within(dialog).getByText("Se ha modificado después de la última firma"),
    ).toBeInTheDocument();
  });

  it("lists one row per problem, findings first, and leaves the valid signatures out", async () => {
    const { user, sign } = await readyToSign(
      signerOver(
        {
          signatures: [validSignature, expiredSignature, unknownTypeSignature],
          findings: ["contentAddedOnTop"],
        },
        recordingPresign([]),
      ),
    );

    await user.click(sign);

    const dialog = await screen.findByRole("dialog", { name: "¿Firmar de todos modos?" });
    const rows = within(dialog).getAllByRole("listitem");
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent("Se ha añadido contenido encima de lo firmado");
    expect(rows[1]).toHaveTextContent("Firma 2 · Bruce Wayne");
    expect(rows[2]).toHaveTextContent("Firma 3 · Notaría XYZ");
    expect(within(dialog).queryByText(/Alfred Pennyworth/)).toBeNull();
    expect(within(dialog).queryByText(/firmas que no son válidas/)).toBeNull();
  });

  it("shows a signature of an unknown type as one more row, with no dialog of its own", async () => {
    const { user, sign } = await readyToSign(
      signerOver({ signatures: [unknownTypeSignature] }, recordingPresign([])),
    );

    await user.click(sign);

    const dialog = await screen.findByRole("dialog", { name: "¿Firmar de todos modos?" });
    expect(within(dialog).getByText("Firma 1 · Notaría XYZ")).toBeInTheDocument();
    expect(within(dialog).getByText("rFirma no conoce este tipo de firma")).toBeInTheDocument();
    expect(
      screen.queryByRole("dialog", { name: "Este PDF trae firmas que no entendemos" }),
    ).toBeNull();
  });

  it("lets the bridge cosign once the unknown-type row is accepted", async () => {
    const presigned: SigningOrder[] = [];
    const { user, sign } = await readyToSign(
      signerOver({ signatures: [unknownTypeSignature] }, recordingPresign(presigned)),
    );

    await user.click(sign);
    const dialog = await screen.findByRole("dialog", { name: "¿Firmar de todos modos?" });
    await user.click(within(dialog).getByRole("button", { name: "Firmar de todos modos" }));

    await waitFor(() => expect(presigned).toHaveLength(1));
    expect(presigned[0]?.allowUnregisteredSignatures).toBe(true);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("signs nothing when it is cancelled", async () => {
    const presigned: SigningOrder[] = [];
    const { user, sign } = await readyToSign(
      signerOver({ signatures: [expiredSignature] }, recordingPresign(presigned)),
    );

    await user.click(sign);
    const dialog = await screen.findByRole("dialog", { name: "¿Firmar de todos modos?" });
    await user.click(within(dialog).getByRole("button", { name: "Cancelar" }));

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(presigned).toHaveLength(0);
  });

  it("signs with the exact order already built, when confirmed", async () => {
    const presigned: SigningOrder[] = [];
    const { user, sign } = await readyToSign(
      signerOver({ signatures: [expiredSignature] }, recordingPresign(presigned)),
    );

    await user.click(sign);
    const dialog = await screen.findByRole("dialog", { name: "¿Firmar de todos modos?" });
    await user.click(within(dialog).getByRole("button", { name: "Firmar de todos modos" }));

    await waitFor(() => expect(presigned).toHaveLength(1));
    expect(presigned[0]?.allowUnregisteredSignatures).toBeFalsy();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("signs directly when every signature is valid", async () => {
    const presigned: SigningOrder[] = [];
    const { user, sign } = await readyToSign(
      signerOver(
        { signatures: [validSignature, aSignature({ name: "Selene" })], tone: "information" },
        recordingPresign(presigned),
      ),
    );

    await user.click(sign);

    await waitFor(() => expect(presigned).toHaveLength(1));
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("does not offer to sign a closed document, and says so instead of «ya lo firmaste tú»", async () => {
    const user = userEvent.setup();
    const closingSignature = aSignature({
      name: remembered.holderName,
      idNumber: remembered.idNumber,
      issuer: remembered.issuer,
      certificateSerialNumber: remembered.certificateSerialNumber,
      closesDocument: true,
    });
    const presigned: SigningOrder[] = [];
    renderApp(
      inMemoryRecents(),
      [document("cofirmado.pdf", { placement: aPlacement })],
      pdfsOf({ "cofirmado.pdf": 4 }),
      {},
      { list: async () => [remembered] },
      emptyRubricPicker(),
      signerOver(
        { signatures: [closingSignature], tone: "information", closed: true },
        recordingPresign(presigned),
      ),
    );
    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await within(panel).findByText("El documento no admite más firmas.");
    await within(within(panel).getByRole("combobox")).findByText(remembered.holderName);

    const sign = within(panel).getByRole("button", { name: "Firmar" });
    expect(sign).toBeDisabled();
    expect(sign).toHaveAttribute("title", "El documento no admite más firmas.");
    expect(within(panel).queryByText("Ya lo firmaste tú con este certificado")).toBeNull();
  });
});
