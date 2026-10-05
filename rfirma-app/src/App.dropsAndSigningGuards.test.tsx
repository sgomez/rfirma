import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { aCertificate, document, openPdf, pdfsOf, renderApp } from "./App.testSupport";
import type { DocumentInHand } from "./documents/document";
import { inMemoryRecents } from "./documents/recents";
import type { Placement } from "./placement/pageSets";
import type { Certificate } from "./signing/certificate";
import type { SigningBackend } from "./signing/flow";
import type { PreviousSignature } from "./signing/previousSignatures";
import { NO_PREVIOUS_SIGNATURES } from "./signing/previousSignatures";
import { emptyRubricPicker } from "./signing/rubric";

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
    const { drops } = renderApp({
      recents: inMemoryRecents(),
      documents: [],
      pdfs: pdfsOf({ "factura.pdf": 7 }),
    });

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
    const { drops } = renderApp({
      recents: inMemoryRecents(),
      documents: [],
      pdfs: pdfsOf({ "factura.pdf": 2 }),
    });

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
    const { drops } = renderApp({
      recents: inMemoryRecents(),
      documents: [],
      pdfs: pdfsOf({ "factura.pdf": 2 }),
    });

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
    const { drops } = renderApp({
      recents: inMemoryRecents(),
      documents: [],
      pdfs: pdfsOf({ "factura.pdf": 2 }),
    });

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
    expect(alert).toHaveTextContent("No se ha podido leer el documento");
    expect(alert).toHaveTextContent("Ábrelo de nuevo con «Abrir PDF…»");
    // Y el detalle crudo sigue ahí, sin traducir, para el informe de fallo.
    expect(alert).toHaveTextContent("os error 2");
  });

  /** El aviso habla del documento que hay delante, así que se va con él. */
  it("drops the notice once another document is in front", async () => {
    const user = userEvent.setup();
    const { drops } = renderApp({
      recents: inMemoryRecents(),
      documents: [document("otro.pdf")],
      pdfs: pdfsOf({ "factura.pdf": 2, "otro.pdf": 3 }),
    });
    drops.drop(anOpened("factura.pdf", [], 2));
    await screen.findByRole("alert");

    await openPdf(user);

    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
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

  /** Lo que mandará la sede: se pinta y se firma, pero no se guarda. */
  const fromTheSede = () => document("de-la-sede.pdf", { remembered: false });

  it("paints it in the viewer without leaving it among the recents", async () => {
    const user = userEvent.setup();
    const recents = inMemoryRecents();
    renderApp({ recents, documents: [fromTheSede()], pdfs: pdfsOf({ "de-la-sede.pdf": 4 }) });

    await openPdf(user);

    await screen.findByRole("region", { name: "Panel de firma" });
    expect(screen.getByRole("tab", { name: "de-la-sede.pdf", selected: true })).toBeInTheDocument();
    await expect(recents.list()).resolves.toEqual([]);
  });

  it("leaves no placement behind when the box is put on it", async () => {
    const user = userEvent.setup();
    const recents = inMemoryRecents();
    renderApp({
      recents,
      documents: [fromTheSede()],
      pdfs: pdfsOf({ "de-la-sede.pdf": 4 }),
      settings: {},
      certificates: { list: async () => [remembered] },
    });
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
});

/** El botón de firmar de un documento que no admite más firmas: lo apaga el panel, no el recorrido. */
describe("App, con un documento cerrado a más firmas", () => {
  const remembered: Certificate = { ...aCertificate, remembered: true };

  const aPlacement: Placement = {
    rect: { x0: 250, y0: 50, x1: 450, y1: 100 },
    pages: { only: [1] },
  };

  it("does not offer to sign a closed document, and says so instead of «ya lo firmaste tú»", async () => {
    const user = userEvent.setup();
    const closingSignature: PreviousSignature = {
      name: remembered.holderName,
      idNumber: remembered.idNumber,
      organizationIdentifier: null,
      issuer: remembered.issuer,
      certificateSerialNumber: remembered.certificateSerialNumber,
      signingTime: "2024-01-01T10:00:00.000Z",
      validity: "valid",
      validityReason: null,
      signingDate: null,
      closesDocument: true,
      countersignatures: [],
    };
    const signer: SigningBackend = {
      presign: async () => ({ ok: true, value: { kind: "typedOnScreen" } }),
      sign: async () => ({ ok: true, value: undefined }),
      postsign: async () => ({
        ok: true,
        value: { name: "cofirmado-firmado.pdf", folder: "Documentos", sizeBytes: 1 },
      }),
      padesLowerLeft: async (placement) => [placement.rect[0], placement.rect[1]],
      previousSignatures: async () => ({
        ...NO_PREVIOUS_SIGNATURES,
        signatures: [closingSignature],
        closed: true,
      }),
      signedDocumentSignatures: async () => NO_PREVIOUS_SIGNATURES,
      discard: async () => {},
    };
    renderApp({
      recents: inMemoryRecents(),
      documents: [document("cofirmado.pdf", { placement: aPlacement })],
      pdfs: pdfsOf({ "cofirmado.pdf": 4 }),
      settings: {},
      certificates: { list: async () => [remembered] },
      rubrics: emptyRubricPicker(),
      signer,
    });
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
