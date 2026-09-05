import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";
import type { DocumentInHand } from "./documents/document";
import type { Drop, FakeDocumentDrops } from "./documents/drops";
import { inMemoryDocumentDrops } from "./documents/drops";
import { inMemoryDocumentPicker } from "./documents/picker";
import { inMemoryRecents, type RecentDocument } from "./documents/recents";
import type { Preferences, PreferencesStore } from "./preferences/preferences";
import { inMemoryPreferences } from "./preferences/preferences";
import type { Certificate, CertificateStore } from "./signing/certificate";
import { emptyCertificateStore } from "./signing/certificate";
import { inMemoryDestination, unavailableOpener } from "./signing/destination";
import { type SigningBackend, type SigningOrder, unavailableSigningBackend } from "./signing/flow";
import { emptyRubricPicker, type RubricPicker } from "./signing/rubric";
import { unavailableStampComposer } from "./signing/stampPreview";
import { renderWithCatalog } from "./testing/render";
import { inMemoryVersionCheck, type VersionCheck } from "./updates/newVersion";

/** El destino que contesta el backend mientras la prueba no diga otra cosa. */
const aDestination = () =>
  inMemoryDestination({ folder: "Documentos", name: "contrato-firmado.pdf", writable: true });

import type { PdfDocument, PdfPage, Viewport } from "./viewer/pdf";
import type { Placement } from "./viewer/signatureBox";
import { type PdfSource, unavailablePdfSource } from "./viewer/source";

/**
 * **El documento que se tiene delante**: lo que entra por el diálogo o por el
 * arrastre, y lo que se pinta y se firma. No es la fila (ID-287).
 */
function document(name: string, overrides: Partial<DocumentInHand> = {}): DocumentInHand {
  return {
    // El identificador lo acuña el backend y es opaco: aquí se finge con un
    // prefijo que ninguna ruta tendría, para que nada pueda leerlo como tal.
    id: `id-${name}`,
    name,
    badge: "Unsigned",
    modified: 1_700_000_000,
    placement: null,
    remembered: true,
    ...overrides,
  };
}

/** **La fila que se guarda**: con lo que arranca la bandeja en una prueba. */
function row(name: string, overrides: Partial<RecentDocument> = {}): RecentDocument {
  return {
    id: `id-${name}`,
    name,
    badge: "Unsigned",
    modified: 1_700_000_000,
    lastUsed: 1_700_000_000,
    available: true,
    placement: null,
    ...overrides,
  };
}

const A4 = { width: 595, height: 842 };

/** Un viewport de `pdf.js` sin rotación: escala y voltea el eje Y. */
function viewportAt(scale: number): Viewport {
  return {
    width: A4.width * scale,
    height: A4.height * scale,
    convertToPdfPoint: (x, y) => [x / scale, A4.height - y / scale],
    convertToViewportPoint: (x, y) => [x * scale, (A4.height - y) * scale],
  };
}

/** Un PDF que se deja pintar: `pdf.js` no cabe en `jsdom` (ver `pdf.ts`). */
function aPdfOf(pageCount: number): PdfDocument {
  const pageOf = (number: number): PdfPage => ({
    number,
    rotate: 0,
    view: [0, 0, A4.width, A4.height],
    getViewport: ({ scale }) => viewportAt(scale),
    render: () => ({ promise: Promise.resolve(), cancel: () => {} }),
  });
  return { pageCount, getPage: (number) => Promise.resolve(pageOf(number)) };
}

/** Un origen que abre cada documento con las páginas que se le digan. */
function pdfsOf(pages: Record<string, number>): PdfSource {
  return {
    open: async (document) => {
      const pageCount = pages[document.name];
      if (pageCount === undefined) {
        return { ok: false, failure: { situation: "documentUnreadable", detail: "roto" } };
      }
      return { ok: true, pdf: aPdfOf(pageCount), sizeBytes: 2_400_000 };
    },
  };
}

/**
 * Un PDF de tamaños mezclados: cada página trae su propio `view`. Es lo que
 * hace falta para que `correctPositionSignature` se coma alguna en silencio
 * (ID-105) y para probarlo hace falta más de un tamaño en el mismo documento.
 */
function aPdfWithViews(views: readonly (readonly [number, number, number, number])[]): PdfDocument {
  const pageOf = (number: number): PdfPage => {
    const view = views[number - 1];
    if (view === undefined) throw new Error(`no hay view para la página ${number}`);
    return {
      number,
      rotate: 0,
      view,
      getViewport: ({ scale }) => viewportAt(scale),
      render: () => ({ promise: Promise.resolve(), cancel: () => {} }),
    };
  };
  return { pageCount: views.length, getPage: (number) => Promise.resolve(pageOf(number)) };
}

/** Un origen que abre `name` con las `views` que se le den, tamaños mezclados incluidos. */
function pdfsWithViews(
  name: string,
  views: readonly (readonly [number, number, number, number])[],
): PdfSource {
  return {
    open: async (opened) => {
      if (opened.name !== name) {
        return { ok: false, failure: { situation: "documentUnreadable", detail: "roto" } };
      }
      return { ok: true, pdf: aPdfWithViews(views), sizeBytes: 2_400_000 };
    },
  };
}

const aCertificate: Certificate = {
  id: "0123456789abcdef0123456789abcdef",
  label: "Firma",
  holderName: "Ada Lovelace Byron",
  idNumber: "99999999R",
  issuer: "AC FNMT Usuarios",
  store: "card",
  status: { kind: "valid", notAfter: 1_894_752_000 },
  remembered: false,
};

/**
 * Un almacén que **rechaza** las primeras `failures` búsquedas y a partir de
 * ahí devuelve lo que se le diga: es el token que no carga y que, arreglado el
 * problema, sí carga al volver a buscar.
 */
function failingCertificateStore(failures: number, then: readonly Certificate[] = []) {
  let left = failures;
  const store: CertificateStore = {
    ...emptyCertificateStore(),
    list: async () => {
      if (left > 0) {
        left -= 1;
        // La forma que rechaza `invoke` cuando Rust ya clasificó el fallo.
        throw { situation: "moduleNotFound", detail: "CKR_GENERAL_ERROR" };
      }
      return then;
    },
  };
  return store;
}

function renderApp(
  recents = inMemoryRecents(),
  documents: DocumentInHand[] = [],
  pdfs: PdfSource = unavailablePdfSource(),
  settings: Partial<Preferences> = {},
  certificates: Partial<CertificateStore> = {},
  rubrics: RubricPicker = emptyRubricPicker(),
  signer: SigningBackend = unavailableSigningBackend(),
  invoked: Drop | null = null,
  drops: FakeDocumentDrops = inMemoryDocumentDrops(invoked),
  versions: VersionCheck = inMemoryVersionCheck(),
) {
  const preferences = inMemoryPreferences(
    {
      theme: "system",
      destination: "Documentos",
      offersOriginalFolder: false,
      rememberVisibleSignature: true,
      rememberActivity: true,
      notifyNewVersion: true,
      ...settings,
    },
    () => void recents.clear(),
  );
  renderWithCatalog(
    <App
      recents={recents}
      picker={inMemoryDocumentPicker(documents)}
      drops={drops}
      pdfs={pdfs}
      preferences={preferences}
      destinations={aDestination()}
      certificates={{ ...emptyCertificateStore(), ...certificates }}
      rubrics={rubrics}
      stamps={unavailableStampComposer()}
      signer={signer}
      opener={unavailableOpener()}
      versions={versions}
      menuAnchor="header"
    />,
  );
  return { recents, preferences, drops };
}

/**
 * La zona de soltar **de la bandeja**. Desde que el visor existe hay dos con el
 * mismo rótulo —la de la bandeja y la del visor vacío—, y las dos fichas las
 * piden: `bandeja-de-documentos.md` y `visor-de-documento.md`.
 */
function trayDropZone() {
  const tray = screen.getByRole("region", { name: "Bandeja de documentos" });
  return within(tray).getByRole("button", { name: "Arrastra un PDF o pulsa para abrirlo" });
}

// Grada A: la aplicación entera, con los cinco puertos en memoria.
describe("App", () => {
  /**
   * El tema es lo único de los ajustes que se pinta **fuera** del árbol de
   * React: los tokens de color cuelgan de `<html>`.
   */
  it("puts the remembered theme on the document as soon as the settings are read", async () => {
    renderApp(inMemoryRecents(), [], unavailablePdfSource(), { theme: "dark" });

    await waitFor(() =>
      expect(globalThis.document.documentElement).toHaveAttribute("data-theme", "dark"),
    );
  });

  it("leaves the theme to the desktop when nothing was chosen", async () => {
    globalThis.document.documentElement.setAttribute("data-theme", "dark");
    renderApp(inMemoryRecents(), [], unavailablePdfSource(), { theme: "system" });

    await waitFor(() =>
      expect(globalThis.document.documentElement).not.toHaveAttribute("data-theme"),
    );
  });

  /**
   * El criterio del #128: mover o borrar el fichero original no pierde la
   * rúbrica, sigue ahí a la siguiente sesión. Lo que se comprueba aquí es la
   * mitad de la ventana —lee lo que el almacén ya tenía adoptado al arrancar,
   * sin que nadie vuelva a elegirla—, no el disco: eso lo prueba
   * `RubricStore::stored` en Rust.
   */
  it("shows the rubric a previous session already adopted, without choosing it again", async () => {
    const user = userEvent.setup();
    const rubrics: RubricPicker = {
      choose: async () => null,
      stored: async () => ({ dataUrl: "data:image/jpeg;base64,/9j/", width: 200, height: 80 }),
    };
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      // Con certificado: desde el ID-108 el bloque entero de firma visible
      // —la rúbrica incluida— está apagado hasta que hay con qué firmar.
      failingCertificateStore(0, [aCertificate]),
      rubrics,
    );

    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });

    expect(
      await within(panel).findByRole("img", { name: "Tu rúbrica, tal como se estampará" }),
    ).toBeInTheDocument();
  });

  /**
   * Un ajuste que el disco no acepta **no se queda puesto**: la ventana
   * volvería a abrirse con el valor anterior, así que enseñarlo cambiado sería
   * mentir sobre la sesión siguiente.
   */
  it("puts a setting back when the disk refuses to keep it", async () => {
    const user = userEvent.setup();
    const refused = vi.fn(async () => {
      throw new Error("no se deja escribir");
    });
    const preferences: PreferencesStore = {
      read: async () => ({
        theme: "system",
        destination: "Documentos",
        offersOriginalFolder: false,
        rememberVisibleSignature: true,
        rememberActivity: true,
        notifyNewVersion: true,
      }),
      save: refused,
      forgetActivity: async () => {},
      chooseFolder: async () => null,
    };
    renderWithCatalog(
      <App
        recents={inMemoryRecents()}
        picker={inMemoryDocumentPicker([])}
        drops={inMemoryDocumentDrops()}
        pdfs={unavailablePdfSource()}
        preferences={preferences}
        destinations={aDestination()}
        certificates={emptyCertificateStore()}
        rubrics={emptyRubricPicker()}
        stamps={unavailableStampComposer()}
        signer={unavailableSigningBackend()}
        opener={unavailableOpener()}
        versions={inMemoryVersionCheck()}
        menuAnchor="header"
      />,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    const remember = await screen.findByRole("switch", {
      name: /Recordar la última configuración de firma visible/,
    });
    await user.click(remember);

    // Se intentó guardar —así que el clic sí llegó— y aun así el interruptor
    // vuelve a estar como estaba, y ahora además se dice, en su sección.
    await waitFor(() => expect(refused).toHaveBeenCalledOnce());
    expect(remember).toHaveAttribute("aria-checked", "true");
    const notice = await screen.findByRole("alert");
    expect(notice).toHaveTextContent("No hemos podido guardar el ajuste");
    expect(screen.getByRole("region", { name: "Firma" })).toContainElement(notice);
    expect(screen.getByText("no se deja escribir")).toBeInTheDocument();
  });

  it("opens a document from the tray and shows its badge in the header", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")]);

    await user.click(trayDropZone());

    expect(await screen.findByText("factura.pdf")).toBeInTheDocument();
    expect(screen.getByRole("banner")).toHaveTextContent("Sin firmar");
  });

  /**
   * El recorrido entero del #82, contado por lo que se ve y no por las órdenes
   * que se llamaron (TD-15): se elige un PDF y queda pintado, con su nombre y
   * sus páginas en el panel, y anotado en la bandeja como no firmado (ID-71).
   */
  it("paints the chosen document in the viewer and annotates it in the tray", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")], pdfsOf({ "factura.pdf": 7 }));

    await user.click(trayDropZone());

    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    expect(within(panel).getByText("factura.pdf")).toBeInTheDocument();
    expect(within(panel).getByText(/^7 páginas/)).toBeInTheDocument();
    const tray = screen.getByRole("region", { name: "Bandeja de documentos" });
    expect(within(tray).getByText("Sin firmar")).toBeInTheDocument();
    // El visor vacío tenía su propia zona de soltar; con el documento pintado
    // solo queda la de la bandeja.
    expect(
      screen.getAllByRole("button", { name: "Arrastra un PDF o pulsa para abrirlo" }),
    ).toHaveLength(1);
  });

  /**
   * El cuelgue del #97: la promesa rechazada no la recogía nadie y la ficha se
   * quedaba en «Buscando certificados…» para siempre, con el rechazo saliendo
   * en el registro como *unhandled rejection*.
   */
  it("names the failure and offers to look again when the certificate search rejects", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      failingCertificateStore(1),
    );

    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });

    await waitFor(() =>
      expect(within(panel).queryByText("Buscando certificados…")).not.toBeInTheDocument(),
    );
    // El mensaje es el del fallo clasificado, y **no** el de «no hay ninguno».
    expect(within(panel).getByRole("alert")).toHaveTextContent(
      "No hemos podido cargar el módulo de la tarjeta",
    );
    expect(within(panel).queryByText("No hemos encontrado ningún certificado")).toBeNull();
    expect(within(panel).getByRole("button", { name: "Volver a buscar" })).toBeInTheDocument();
    // El fallo se queda dentro de la ficha del certificado: el documento sigue
    // pintado y el visor no se entera.
    expect(within(panel).getByText("factura.pdf")).toBeInTheDocument();
  });

  it("loads the list when looking again with the problem already solved", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      failingCertificateStore(1, [aCertificate]),
    );
    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const retry = await within(panel).findByRole("button", { name: "Volver a buscar" });

    await user.click(retry);

    expect(await within(panel).findByText("Ada Lovelace Byron")).toBeInTheDocument();
  });

  /**
   * Con varios certificados **no hay preselección**, y elegir una fila deja
   * puesto ese certificado y no el primero de la lista. La colisión de
   * etiquetas —dos filas con el mismo `CKA_LABEL`— la prueban el desplegable
   * (grada A) y `tests/pkcs11_token.rs` (grada B); aquí lo que se comprueba es
   * el recorrido entero de la ventana.
   */
  it("chooses no certificate by itself and takes the one that is picked", async () => {
    const user = userEvent.setup();
    const other: Certificate = { ...aCertificate, id: "otra", holderName: "Grace Hopper Murray" };
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      { list: async () => [aCertificate, other] },
    );
    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const trigger = await within(panel).findByRole("combobox", { name: "Certificado" });

    expect(trigger).toHaveTextContent("Elegir certificado");
    expect(within(panel).getByRole("button", { name: "Firmar documento" })).toBeDisabled();

    await user.click(trigger);
    // La lista vive en un portal, fuera de `panel` (ID-308): se busca en todo
    // el documento, no dentro del panel.
    const rows = screen.getAllByRole("option");
    const second = rows[1];
    if (second === undefined) throw new Error("la lista tenia que traer dos filas");
    await user.click(second);

    expect(trigger).toHaveTextContent("Grace Hopper Murray");
    // Con el interruptor de firma visible encendido y sin recuadro colocado,
    // firmar sigue apagado: es el otro «no» del ID-93, y no el del certificado.
    expect(within(panel).getByRole("button", { name: "Firmar documento" })).toBeDisabled();
  });

  /**
   * Quien tiene cuatro certificados los elige **una vez**, no cada día: el que
   * se usó en la última firma sale ya puesto, sin pedir el PIN (#110). Quién es
   * ese lo decide el backend —las coordenadas del token no cruzan la
   * frontera—; aquí llega marcada la fila.
   */
  it("starts with the certificate that was used the last time already chosen", async () => {
    const user = userEvent.setup();
    const used: Certificate = {
      ...aCertificate,
      id: "otra",
      holderName: "Grace Hopper Murray",
      remembered: true,
    };
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      { list: async () => [aCertificate, used] },
    );

    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const trigger = await within(panel).findByRole("combobox", { name: "Certificado" });

    expect(trigger).toHaveTextContent("Grace Hopper Murray");
    // Con el interruptor de firma visible encendido y sin recuadro colocado,
    // firmar sigue apagado: es el otro «no» del ID-93, y no el del certificado.
    expect(within(panel).getByRole("button", { name: "Firmar documento" })).toBeDisabled();
  });

  /**
   * Y el recordado que ya no está —tarjeta fuera, perfil borrado— deja el panel
   * en «Sin certificado» **sin ruido**: no viene marcada ninguna fila, y eso no
   * es un error que contar (ADR-0010).
   */
  it("falls back to no certificate when the remembered one is gone, without an error", async () => {
    const user = userEvent.setup();
    const other: Certificate = { ...aCertificate, id: "otra", holderName: "Grace Hopper Murray" };
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      { list: async () => [aCertificate, other] },
    );

    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const trigger = await within(panel).findByRole("combobox", { name: "Certificado" });

    expect(trigger).toHaveTextContent("Elegir certificado");
    expect(within(panel).queryByRole("alert")).not.toBeInTheDocument();
  });

  /** El recordado no escapa a la regla de «nunca se preselecciona un
   * certificado no utilizable»: si caducó desde la última firma, el
   * desplegable arranca sin elección, igual que si no hubiera recordado
   * ninguno (#197). */
  it("does not preselect the remembered certificate when it has expired since", async () => {
    const user = userEvent.setup();
    const expired: Certificate = {
      ...aCertificate,
      id: "otra",
      holderName: "Grace Hopper Murray",
      status: { kind: "expired", notAfter: 0 },
      remembered: true,
    };
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      { list: async () => [aCertificate, expired] },
    );

    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const trigger = await within(panel).findByRole("combobox", { name: "Certificado" });

    expect(trigger).toHaveTextContent("Elegir certificado");
  });

  /** «Con uno solo se elige solo» gana una excepción: si ese único no sirve,
   * el desplegable arranca sin elección (#197). */
  it("does not preselect the sole certificate when it cannot be used", async () => {
    const user = userEvent.setup();
    const expired: Certificate = {
      ...aCertificate,
      status: { kind: "expired", notAfter: 0 },
    };
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      { list: async () => [expired] },
    );

    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const trigger = await within(panel).findByRole("combobox", { name: "Certificado" });

    expect(trigger).toHaveTextContent("Elegir certificado");
  });

  it("names the error of a PDF it cannot read instead of leaving an empty viewer", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("corrupto.pdf")], pdfsOf({}));

    await user.click(trayDropZone());

    expect(await screen.findByRole("alert")).toHaveTextContent("No hemos podido leer el documento");
  });

  it("repaints a document when its tray row is chosen again, one after another", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("primero.pdf"), document("segundo.pdf")],
      pdfsOf({ "primero.pdf": 2, "segundo.pdf": 5 }),
    );
    await user.click(trayDropZone());
    await screen.findByRole("region", { name: "Panel de firma" });

    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await waitFor(() => expect(within(panel).getByText("segundo.pdf")).toBeInTheDocument());

    await user.click(screen.getByRole("button", { name: /primero\.pdf/ }));

    await waitFor(() => expect(within(panel).getByText("primero.pdf")).toBeInTheDocument());
    expect(within(panel).getByText(/^2 páginas/)).toBeInTheDocument();
  });

  it("changes nothing when the dialog is closed without choosing", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")], pdfsOf({ "factura.pdf": 3 }));
    await user.click(trayDropZone());
    await screen.findByRole("region", { name: "Panel de firma" });

    // El selector en memoria se agota tras el primero, y a partir de ahí se
    // comporta como una cancelación (ID-73).
    await user.click(trayDropZone());

    const panel = screen.getByRole("region", { name: "Panel de firma" });
    expect(within(panel).getByText("factura.pdf")).toBeInTheDocument();
    const tray = screen.getByRole("region", { name: "Bandeja de documentos" });
    expect(within(tray).getAllByText("factura.pdf")).toHaveLength(1);
  });

  it("opens Preferences from the menu, over the window and without unmounting it", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents([row("a.pdf")]));
    await screen.findByText("a.pdf");

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));

    expect(await screen.findByRole("dialog", { name: "Preferencias" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Bandeja de documentos" })).toBeInTheDocument();
    expect(screen.getByText("a.pdf")).toBeInTheDocument();
  });

  /**
   * Preferencias y el desplegable de la firma salen del **mismo** listado: lo
   * que se acaba de instalar aparece en los dos sin volver a arrancar (ID-198).
   */
  it("lists a just installed certificate in Preferences", async () => {
    const user = userEvent.setup();
    const p12: Certificate = { ...aCertificate, id: "p12", store: "installed" };
    let found: readonly Certificate[] = [];
    renderApp(
      inMemoryRecents(),
      [],
      unavailablePdfSource(),
      {},
      {
        list: async () => found,
        install: async () => {
          found = [p12];
          return true;
        },
      },
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    const certificates = await screen.findByRole("region", { name: "Certificados" });
    expect(certificates).toHaveTextContent("Todavía no has instalado ninguno");

    await user.click(within(certificates).getByRole("button", { name: "Añadir…" }));
    await user.click(screen.getByRole("button", { name: "Continuar" }));

    expect(await within(certificates).findByText("Ada Lovelace Byron")).toBeInTheDocument();
  });

  it("takes a removed certificate out of the list", async () => {
    const user = userEvent.setup();
    const p12: Certificate = { ...aCertificate, id: "p12", store: "installed" };
    let found: readonly Certificate[] = [p12];
    renderApp(
      inMemoryRecents(),
      [],
      unavailablePdfSource(),
      {},
      {
        list: async () => found,
        remove: async () => {
          found = [];
        },
      },
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    const certificates = await screen.findByRole("region", { name: "Certificados" });
    await within(certificates).findByText("Ada Lovelace Byron");

    await user.click(
      within(certificates).getByRole("button", {
        name: "Quitar el certificado de Ada Lovelace Byron",
      }),
    );

    expect(
      await within(certificates).findByText("Todavía no has instalado ninguno"),
    ).toBeInTheDocument();
  });

  it("opens About from the menu", async () => {
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Acerca de rFirma" }));

    expect(screen.getByText(/Proyecto independiente/)).toBeInTheDocument();
  });

  it("empties the tray when Remember my activity is turned off", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents([row("a.pdf")]));
    await screen.findByText("a.pdf");

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    await user.click(await screen.findByRole("switch", { name: /Recordar mi actividad/ }));
    await user.click(screen.getByRole("button", { name: "Borrar y apagar" }));

    await waitFor(() => expect(screen.queryByText("a.pdf")).not.toBeInTheDocument());
    expect(
      screen.getByText("Aquí aparecerán los documentos que vayas firmando"),
    ).toBeInTheDocument();
  });

  /**
   * La bandeja se vacía **aunque el borrado del disco falle** —lo que promete
   * el rótulo es que dejen de estar— y el fallo se cuenta en Privacidad, que es
   * el otro `catch {}` vacío que el ID-70 llena.
   */
  it("empties the tray and says the recents are still saved when the disk refuses", async () => {
    const user = userEvent.setup();
    const recents = inMemoryRecents([row("a.pdf")]);
    const preferences: PreferencesStore = {
      read: async () => ({
        theme: "system",
        destination: "Documentos",
        offersOriginalFolder: false,
        rememberVisibleSignature: true,
        rememberActivity: true,
        notifyNewVersion: true,
      }),
      save: async () => {},
      forgetActivity: async () => {
        throw new Error("no se deja borrar");
      },
      chooseFolder: async () => null,
    };
    renderWithCatalog(
      <App
        recents={recents}
        picker={inMemoryDocumentPicker([])}
        drops={inMemoryDocumentDrops()}
        pdfs={unavailablePdfSource()}
        preferences={preferences}
        destinations={aDestination()}
        certificates={emptyCertificateStore()}
        rubrics={emptyRubricPicker()}
        stamps={unavailableStampComposer()}
        signer={unavailableSigningBackend()}
        opener={unavailableOpener()}
        versions={inMemoryVersionCheck()}
        menuAnchor="header"
      />,
    );
    await screen.findByText("a.pdf");

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    await user.click(await screen.findByRole("button", { name: "Vaciar la lista" }));

    const notice = await screen.findByRole("alert");
    expect(notice).toHaveTextContent("No hemos podido vaciar la lista");
    expect(screen.getByRole("region", { name: "Privacidad" })).toContainElement(notice);
    expect(screen.queryByText("a.pdf")).not.toBeInTheDocument();
  });

  it("stops remembering once Remember my activity is off, not just purges what there was", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents([row("a.pdf")]), [document("factura.pdf")]);
    await screen.findByText("a.pdf");

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    await user.click(await screen.findByRole("switch", { name: /Recordar mi actividad/ }));
    await user.click(screen.getByRole("button", { name: "Borrar y apagar" }));
    await waitFor(() => expect(screen.queryByText("a.pdf")).not.toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    await user.click(trayDropZone());

    expect(screen.getByRole("banner")).toHaveTextContent("Sin firmar");
    expect(screen.queryByText("factura.pdf")).not.toBeInTheDocument();
    expect(
      screen.getByText("Aquí aparecerán los documentos que vayas firmando"),
    ).toBeInTheDocument();
  });
});

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

    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    expect(within(panel).getByText("factura.pdf")).toBeInTheDocument();
    expect(within(panel).getByText(/^7 páginas/)).toBeInTheDocument();
    const tray = screen.getByRole("region", { name: "Bandeja de documentos" });
    expect(within(tray).getByText("Sin firmar")).toBeInTheDocument();
    expect(screen.getByRole("banner")).toHaveTextContent("Sin firmar");
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

  /** ID-306: se abre el primero, y el resto entra igual en Recientes. */
  it("opens the first of several dropped PDFs and lists the rest in the tray", async () => {
    const { drops } = renderApp(inMemoryRecents(), [], pdfsOf({ "factura.pdf": 2 }));

    drops.drop(anOpened("factura.pdf", [document("contrato.pdf")]));

    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    expect(within(panel).getByText("factura.pdf")).toBeInTheDocument();
    const tray = screen.getByRole("region", { name: "Bandeja de documentos" });
    expect(await within(tray).findByText("contrato.pdf")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  /**
   * ID-306: soltar N ficheros deja N filas en la bandeja, no una, y a la vez
   * se dice cuántos se descartaron — las dos cosas del mismo gesto, no dos
   * casos por separado.
   */
  it("drops N files into N tray rows and counts the discarded ones in the same gesture", async () => {
    const { drops } = renderApp(inMemoryRecents(), [], pdfsOf({ "factura.pdf": 2 }));

    drops.drop(anOpened("factura.pdf", [document("contrato.pdf"), document("anexo.pdf")], 2));

    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    expect(within(panel).getByText("factura.pdf")).toBeInTheDocument();
    const tray = screen.getByRole("region", { name: "Bandeja de documentos" });
    expect(await within(tray).findByText("contrato.pdf")).toBeInTheDocument();
    expect(within(tray).getByText("anexo.pdf")).toBeInTheDocument();
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Algunos ficheros no se han añadido",
    );
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

    await user.click(trayDropZone());

    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  });
});

/**
 * ID-105/ID-106: el diálogo de páginas sin sello, justo antes de firmar y
 * gateado en `App.sign` (docs/design/dialogo-paginas-sin-sello.md).
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
      value: { kind: "typedOnScreen" as const, attemptsLeft: null },
    }));
    const signer: SigningBackend = {
      presign,
      sign: async () => ({ ok: true, value: undefined }),
      postsign: async () => ({
        ok: true,
        value: { name: "factura.pdf", folder: "Documentos", sizeBytes: 1 },
      }),
      padesLowerLeft: async (placement) => [placement.rect[0], placement.rect[1]],
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

    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const sign = await within(panel).findByRole("button", { name: "Firmar documento" });
    await waitFor(() => expect(sign).toBeEnabled());

    await user.click(sign);

    // ID-106: el denominador es el conjunto elegido (3), no el documento.
    expect(
      await screen.findByRole("dialog", { name: "Una página se quedará sin sello" }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "El recuadro no cabe en 1 de las 3 páginas que has elegido, más pequeñas que aquella " +
          "sobre la que lo colocaste. El documento se firmará igual y la firma será válida en " +
          "todo él, pero en esas páginas no aparecerá el sello.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("El sello aparecerá en 2 de las 3 páginas elegidas."),
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
      value: { kind: "typedOnScreen" as const, attemptsLeft: null },
    }));
    const signer: SigningBackend = {
      presign,
      sign: async () => ({ ok: true, value: undefined }),
      postsign: async () => ({
        ok: true,
        value: { name: "factura.pdf", folder: "Documentos", sizeBytes: 1 },
      }),
      padesLowerLeft: async (placement) => [placement.rect[0], placement.rect[1]],
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

    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const sign = await within(panel).findByRole("button", { name: "Firmar documento" });
    await waitFor(() => expect(sign).toBeEnabled());
    await user.click(sign);
    await screen.findByRole("dialog", { name: "Una página se quedará sin sello" });

    await user.click(screen.getByRole("button", { name: "Firmar de todos modos" }));

    await waitFor(() => expect(presign).toHaveBeenCalledOnce());
    const order = presign.mock.calls[0]?.[0];
    expect(order?.placement.pages).toEqual({ only: [1, 2, 3] });
    expect(screen.queryByRole("dialog", { name: /sello/ })).not.toBeInTheDocument();
  });

  it("does not appear when every chosen page keeps its seal", async () => {
    const user = userEvent.setup();
    const presign = vi.fn(async () => ({
      ok: true as const,
      value: { kind: "typedOnScreen" as const, attemptsLeft: null },
    }));
    const signer: SigningBackend = {
      presign,
      sign: async () => ({ ok: true, value: undefined }),
      postsign: async () => ({
        ok: true,
        value: { name: "factura.pdf", folder: "Documentos", sizeBytes: 1 },
      }),
      padesLowerLeft: async (placement) => [placement.rect[0], placement.rect[1]],
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

    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const sign = await within(panel).findByRole("button", { name: "Firmar documento" });
    await waitFor(() => expect(sign).toBeEnabled());

    await user.click(sign);

    await waitFor(() => expect(presign).toHaveBeenCalledOnce());
    expect(screen.queryByRole("dialog", { name: /sello/ })).not.toBeInTheDocument();
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

  it("paints it in the viewer without leaving a row in the tray", async () => {
    const user = userEvent.setup();
    const recents = inMemoryRecents();
    renderApp(recents, [fromTheSede()], pdfsOf({ "de-la-sede.pdf": 4 }));

    await user.click(trayDropZone());

    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    expect(within(panel).getByText("de-la-sede.pdf")).toBeInTheDocument();
    expect(within(panel).getByText(/^4 páginas/)).toBeInTheDocument();
    const tray = screen.getByRole("region", { name: "Bandeja de documentos" });
    expect(within(tray).queryByText("de-la-sede.pdf")).not.toBeInTheDocument();
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
    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await within(panel).findByText("Colocación");

    await user.click(within(panel).getByRole("radio", { name: /Todas las páginas/ }));

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
      value: { kind: "typedOnScreen" as const, attemptsLeft: null },
    }));
    const signer: SigningBackend = {
      presign,
      sign: async () => ({ ok: true, value: undefined }),
      postsign: async () => ({
        ok: true,
        value: { name: "de-la-sede-firmado.pdf", folder: "Documentos", sizeBytes: 1 },
      }),
      padesLowerLeft: async (placement) => [placement.rect[0], placement.rect[1]],
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

    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const sign = await within(panel).findByRole("button", { name: "Firmar documento" });
    await waitFor(() => expect(sign).toBeEnabled());
    await user.click(sign);

    await waitFor(() => expect(presign).toHaveBeenCalledOnce());
    await expect(recents.list()).resolves.toEqual([]);
  });
});

/**
 * El bloque «Colocación», con el visor y el panel a la vez (#185, #188).
 *
 * Vive en la grada A y no en el panel porque los tres caminos que colocan
 * —arrastre, pastilla y campo— acaban en el mismo recuadro **solo si los dos
 * componentes están montados**: el panel nombra páginas y no sabe dónde cae el
 * rectángulo, y el visor pone el rectángulo sin saber cuál de las tres opciones
 * manda. Probado por separado, cada uno pasaba en verde con el fallo dentro.
 */
describe("App · Colocación", () => {
  const remembered: Certificate = { ...aCertificate, remembered: true };

  /** Abre el documento y espera al bloque «Colocación» ya pintado. */
  async function openPlacing() {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 8 }),
      {},
      { list: async () => [remembered] },
    );
    await user.click(trayDropZone());
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await within(panel).findByText("Colocación");
    return { user, panel };
  }

  const box = () => screen.queryByRole("application", { name: "Recuadro de la firma visible" });
  const pill = () => screen.getByRole("button", { name: "Sellar esta página" });

  it("places the box on its standard spot when a range is typed, with nothing placed yet", async () => {
    const { user, panel } = await openPlacing();

    await user.click(within(panel).getByRole("radio", { name: /Estas páginas/ }));
    await user.type(within(panel).getByLabelText("Páginas donde se sella"), "1");

    // El recuadro cae abajo a la derecha sin que nadie lo haya arrastrado: es
    // el ID-102 pedido desde el panel, que es lo que el #185 no hacía.
    expect(box()).not.toBeNull();
  });

  it("places the box when «all the pages» is chosen, with nothing placed yet", async () => {
    const { user, panel } = await openPlacing();

    await user.click(within(panel).getByRole("radio", { name: /Todas las páginas/ }));

    expect(box()).not.toBeNull();
  });

  it("keeps the pill saying the same thing on every option while nothing is placed", async () => {
    const { user, panel } = await openPlacing();

    expect(pill()).toBeInTheDocument();
    // Con el campo vacío «Estas páginas» sigue sin nombrar ninguna página, que
    // es la única opción con la que se puede comparar la pastilla (#188).
    await user.click(within(panel).getByRole("radio", { name: /Estas páginas/ }));

    expect(pill()).toBeInTheDocument();
  });

  it("replaces the sealed page instead of adding to it under «one page only»", async () => {
    const { user, panel } = await openPlacing();

    await user.click(pill());
    await user.click(screen.getByRole("button", { name: "Página siguiente" }));
    await user.click(pill());

    // Ni el aviso del recuadro repetido —que solo aparece con más de una— ni un
    // conjunto de dos: esa opción nombra una página y nada más.
    expect(within(panel).queryByText(/El mismo recuadro/)).toBeNull();
    expect(within(panel).getByText("Página 2")).toBeInTheDocument();
  });

  it("gives each option its own set, so going back brings what was left there", async () => {
    const { user, panel } = await openPlacing();
    const field = () => within(panel).getByLabelText("Páginas donde se sella");

    await user.click(pill());
    await user.click(within(panel).getByRole("radio", { name: /Estas páginas/ }));
    // Se estrena sembrada de la anterior, que es lo que pide la ficha.
    expect(field()).toHaveValue("1");

    await user.clear(field());
    await user.type(field(), "2,5");
    await user.click(within(panel).getByRole("radio", { name: /Solo 1 página/ }));

    // La suya, la 1, y no la más baja del conjunto de al lado (#188).
    expect(within(panel).getByText("Página 1")).toBeInTheDocument();

    await user.click(within(panel).getByRole("radio", { name: /Estas páginas/ }));

    expect(field()).toHaveValue("2,5");
  });
});

/**
 * ID-108: sin certificado utilizable no hay sello que dibujar, y sin sello no
 * hay recuadro. El panel lo cumplía desde siempre —apaga su bloque entero y dice
 * «Elige un certificado para colocar la firma visible»—, pero el visor tenía su
 * propia copia del estado y no lo miraba: ofrecía sellar y dejaba trazar (#190).
 */
describe("App, sin un certificado elegido todavía", () => {
  /** Pulsar, mover y soltar sobre la hoja: el gesto que coloca el recuadro. */
  function traceOverSheet() {
    const sheet = screen.getByRole("document", { name: "Hoja del documento" });
    fireEvent.pointerDown(sheet, { pointerId: 1, button: 0, clientX: 100, clientY: 100 });
    fireEvent.pointerMove(sheet, { pointerId: 1, clientX: 300, clientY: 200 });
    fireEvent.pointerUp(sheet, { pointerId: 1, clientX: 300, clientY: 200 });
  }

  it("neither offers to seal the page nor lets the sheet be traced", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")], pdfsOf({ "factura.pdf": 3 }));

    await user.click(trayDropZone());
    await screen.findByRole("document", { name: "Hoja del documento" });

    expect(screen.getByText("Elige un certificado para colocar la firma visible")).toBeVisible();
    expect(screen.queryByRole("button", { name: "Sellar esta página" })).not.toBeInTheDocument();

    traceOverSheet();

    expect(screen.queryByRole("application")).not.toBeInTheDocument();
  });

  it("lets the sheet be traced as soon as a certificate is chosen", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 3 }),
      {},
      { list: async () => [aCertificate] },
    );

    await user.click(trayDropZone());
    await screen.findByRole("document", { name: "Hoja del documento" });
    const panel = screen.getByRole("region", { name: "Panel de firma" });
    await user.click(await within(panel).findByRole("combobox", { name: "Certificado" }));
    // La lista vive en un portal, fuera de `panel` (ID-308).
    await user.click(screen.getAllByRole("option")[0] as HTMLElement);

    traceOverSheet();

    expect(
      screen.getByRole("application", { name: "Recuadro de la firma visible" }),
    ).toBeInTheDocument();
  });
});

/**
 * **La invocación desde fuera** (ID-157…ID-159): `rfirma documento.pdf`. Lo
 * que el doble entrega por `pending` es lo mismo que devuelve `read_invocation`
 * en Rust, y desemboca en la misma ventana que el arrastre — que es justo lo
 * que estas dos pruebas comprueban: **no hay una segunda interfaz**.
 */
describe("App, invocada con un documento", () => {
  it("opens the invoked PDF in the full window, just like a dropped one", async () => {
    renderApp(
      inMemoryRecents(),
      [],
      pdfsOf({ "contrato.pdf": 3 }),
      {},
      emptyCertificateStore(),
      emptyRubricPicker(),
      unavailableSigningBackend(),
      { document: document("contrato.pdf"), alsoEntering: [], failure: null, discarded: 0 },
    );

    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    expect(within(panel).getByText("contrato.pdf")).toBeInTheDocument();
    expect(within(panel).getByText(/^3 páginas/)).toBeInTheDocument();
  });

  /**
   * `pending()` es una lectura que **consume**, y el efecto que la pide se
   * rehace mientras la llamada está en vuelo: `<StrictMode>` lo hace en
   * desarrollo, y en producción lo hace la lectura asíncrona de los ajustes
   * cuando «Recordar mi actividad» viene apagado —cambia la identidad de
   * `accept`—. Si la entrega dependiera del ciclo de vida del efecto, la
   * respuesta llegaría a un efecto ya limpiado y el documento invocado
   * desaparecería sin ningún aviso.
   *
   * Por eso el doble no contesta solo: la prueba deja que el efecto se rehaga
   * con la lectura en vuelo y la contesta después.
   */
  it("delivers the invoked PDF when the effect remounts while the read is in flight", async () => {
    let answer: (invoked: Drop | null) => void = () => {};
    const base = inMemoryDocumentDrops();
    let asked = 0;
    const drops: FakeDocumentDrops = {
      ...base,
      pending: () => {
        asked += 1;
        return new Promise<Drop | null>((resolve) => {
          answer = resolve;
        });
      },
    };

    renderApp(
      inMemoryRecents(),
      [],
      pdfsOf({ "contrato.pdf": 3 }),
      { rememberActivity: false },
      emptyCertificateStore(),
      emptyRubricPicker(),
      unavailableSigningBackend(),
      null,
      drops,
    );

    // Los ajustes ya han llegado, así que el efecto se ha rehecho: la lectura
    // de la invocación sigue viva y no se ha vuelto a pedir.
    await act(async () => {
      await Promise.resolve();
    });
    expect(asked).toBe(1);

    await act(async () => {
      answer({ document: document("contrato.pdf"), alsoEntering: [], failure: null, discarded: 0 });
    });

    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    expect(within(panel).getByText("contrato.pdf")).toBeInTheDocument();
  });

  /** ID-158: no arranca ningún modo especial, abre la ventana y lo dice. */
  it("opens the normal window and says so when the argument is not a PDF", async () => {
    renderApp(
      inMemoryRecents(),
      [],
      unavailablePdfSource(),
      {},
      emptyCertificateStore(),
      emptyRubricPicker(),
      unavailableSigningBackend(),
      {
        document: null,
        alsoEntering: [],
        failure: { situation: "notAPdf", detail: "el fichero no es un PDF" },
        discarded: 0,
      },
    );

    expect(await screen.findByRole("alert")).toHaveTextContent("Ese fichero no es un PDF");
    expect(screen.getByRole("region", { name: "Bandeja de documentos" })).toBeInTheDocument();
  });

  /**
   * El aviso de versión nueva, que es el primer inquilino de la franja
   * (ID-181, ID-207). Se comprueba desde la aplicación entera porque lo que
   * decide el ticket es **dónde** notifica rFirma: bajo la cabecera y sin
   * modal.
   */
  describe("the new-version notice", () => {
    const withVersionCheck = (versions: VersionCheck) =>
      renderApp(
        inMemoryRecents(),
        [],
        unavailablePdfSource(),
        {},
        emptyCertificateStore(),
        emptyRubricPicker(),
        unavailableSigningBackend(),
        null,
        inMemoryDocumentDrops(),
        versions,
      );

    it("shows a strip under the header, and nothing modal, when there is a new version", async () => {
      withVersionCheck(inMemoryVersionCheck({ version: "0.4.1" }));

      const strip = await screen.findByRole("status");
      expect(strip).toHaveTextContent("Hay una versión nueva de rFirma: 0.4.1");
      // Nada modal: ni diálogo encima ni ventana atenuada.
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(screen.getByRole("region", { name: "Bandeja de documentos" })).toBeInTheDocument();
    });

    it("says nothing at all when there is no new version", async () => {
      withVersionCheck(inMemoryVersionCheck());

      await screen.findByRole("region", { name: "Bandeja de documentos" });
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });

    /**
     * «Avisarme cuando haya una versión nueva» apagado (ID-180): la
     * comprobación sigue corriendo, pero la franja no se monta.
     */
    it("says nothing when Avisarme cuando haya una versión nueva is turned off", async () => {
      renderApp(
        inMemoryRecents(),
        [],
        unavailablePdfSource(),
        { notifyNewVersion: false },
        emptyCertificateStore(),
        emptyRubricPicker(),
        unavailableSigningBackend(),
        null,
        inMemoryDocumentDrops(),
        inMemoryVersionCheck({ version: "0.4.1" }),
      );

      await screen.findByRole("region", { name: "Bandeja de documentos" });
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });

    // Sin red la comprobación ni contesta ni se queja: la ventana se queda
    // como estaba, que es lo que dice el ID-178.
    it("says nothing when the check fails", async () => {
      withVersionCheck({ latest: async () => Promise.reject(new Error("sin red")) });

      await screen.findByRole("region", { name: "Bandeja de documentos" });
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });

    it("takes the user to About, which is where the upgrade instructions are", async () => {
      const user = userEvent.setup();
      withVersionCheck(inMemoryVersionCheck({ version: "0.4.1" }));

      await screen.findByRole("status");
      await user.click(screen.getByRole("button", { name: "Cómo actualizar" }));

      expect(await screen.findByRole("dialog")).toHaveTextContent("rFirma");
    });

    it("is dismissed for good once dismissed", async () => {
      const user = userEvent.setup();
      withVersionCheck(inMemoryVersionCheck({ version: "0.4.1" }));

      await screen.findByRole("status");
      await user.click(screen.getByRole("button", { name: "Descartar" }));

      await waitFor(() => expect(screen.queryByRole("status")).not.toBeInTheDocument());
      // La ventana sigue entera debajo: descartar no navega a ninguna parte.
      expect(screen.getByRole("region", { name: "Bandeja de documentos" })).toBeInTheDocument();
    });
  });
});
