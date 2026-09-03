import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { inMemoryDocumentDrops } from "./documents/drops";
import { inMemoryDocumentPicker } from "./documents/picker";
import { inMemoryRecents, type RecentDocument } from "./documents/recents";
import type { Preferences, PreferencesStore } from "./preferences/preferences";
import { inMemoryPreferences } from "./preferences/preferences";
import type { Certificate, CertificateStore } from "./signing/certificate";
import { emptyCertificateStore } from "./signing/certificate";
import { inMemoryDestination, unavailableOpener } from "./signing/destination";
import { type SigningBackend, unavailableSigningBackend } from "./signing/flow";
import { emptyRubricPicker, type RubricPicker } from "./signing/rubric";
import { emptyLayer2Composer } from "./signing/visibleSignature";
import { renderWithCatalog } from "./testing/render";

/** El destino que contesta el backend mientras la prueba no diga otra cosa. */
const aDestination = () =>
  inMemoryDestination({ folder: "Documentos", name: "contrato-firmado.pdf", writable: true });

import type { PdfDocument, PdfPage, Viewport } from "./viewer/pdf";
import type { Placement } from "./viewer/signatureBox";
import { type PdfSource, unavailablePdfSource } from "./viewer/source";

function document(name: string, overrides: Partial<RecentDocument> = {}): RecentDocument {
  return {
    // El identificador lo acuña el backend y es opaco: aquí se finge con un
    // prefijo que ninguna ruta tendría, para que nada pueda leerlo como tal.
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
      return { ok: true, pdf: aPdfOf(pageCount) };
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
      return { ok: true, pdf: aPdfWithViews(views) };
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
  status: { kind: "valid" },
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
  documents: RecentDocument[] = [],
  pdfs: PdfSource = unavailablePdfSource(),
  settings: Partial<Preferences> = {},
  certificates: CertificateStore = emptyCertificateStore(),
  rubrics: RubricPicker = emptyRubricPicker(),
  signer: SigningBackend = unavailableSigningBackend(),
) {
  const preferences = inMemoryPreferences(
    {
      theme: "system",
      destination: "Documentos",
      rememberVisibleSignature: true,
      rememberActivity: true,
      ...settings,
    },
    () => void recents.clear(),
  );
  const drops = inMemoryDocumentDrops();
  renderWithCatalog(
    <App
      recents={recents}
      picker={inMemoryDocumentPicker(documents)}
      drops={drops}
      pdfs={pdfs}
      preferences={preferences}
      destinations={aDestination()}
      certificates={certificates}
      rubrics={rubrics}
      composer={emptyLayer2Composer()}
      signer={signer}
      opener={unavailableOpener()}
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
      emptyCertificateStore(),
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
        rememberVisibleSignature: true,
        rememberActivity: true,
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
        composer={emptyLayer2Composer()}
        signer={unavailableSigningBackend()}
        opener={unavailableOpener()}
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
    const rows = within(panel).getAllByRole("option");
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
    renderApp(inMemoryRecents([document("a.pdf")]));
    await screen.findByText("a.pdf");

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));

    expect(await screen.findByRole("dialog", { name: "Preferencias" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Bandeja de documentos" })).toBeInTheDocument();
    expect(screen.getByText("a.pdf")).toBeInTheDocument();
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
    renderApp(inMemoryRecents([document("a.pdf")]));
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
    const recents = inMemoryRecents([document("a.pdf")]);
    const preferences: PreferencesStore = {
      read: async () => ({
        theme: "system",
        destination: "Documentos",
        rememberVisibleSignature: true,
        rememberActivity: true,
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
        composer={emptyLayer2Composer()}
        signer={unavailableSigningBackend()}
        opener={unavailableOpener()}
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
    renderApp(inMemoryRecents([document("a.pdf")]), [document("factura.pdf")]);
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
  function anOpened(name: string, ignored = 0) {
    return { document: document(name), failure: null, ignored };
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
      failure: { situation: "notAPdf", detail: "el fichero no es un PDF" },
      ignored: 0,
    });

    expect(await screen.findByRole("alert")).toHaveTextContent("Ese fichero no es un PDF");
  });

  /** ID-70: se abre el primero, y los demás no se callan. */
  it("opens the first PDF of several and says that it only opened that one", async () => {
    const { drops } = renderApp(inMemoryRecents(), [], pdfsOf({ "factura.pdf": 2 }));

    drops.drop(anOpened("factura.pdf", 2));

    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    expect(within(panel).getByText("factura.pdf")).toBeInTheDocument();
    expect(await screen.findByRole("alert")).toHaveTextContent("Solo hemos abierto el primer PDF");
  });

  /**
   * ID-68: el aviso dice **qué hacer**, y lo que hay que hacer es usar el botón
   * de abrir, que sí pasa por el portal. Que este caso exista de verdad —y
   * desde qué carpetas— está medido en
   * `docs/research/arrastre-bajo-el-arenero.md`.
   */
  it("tells what to do when the dropped file cannot be read", async () => {
    const { drops } = renderApp();

    drops.drop({
      document: null,
      failure: {
        situation: "droppedFileUnreadable",
        detail: "No such file or directory (os error 2)",
      },
      ignored: 0,
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
    drops.drop(anOpened("factura.pdf", 2));
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
    const presign = vi.fn(async () => ({ ok: true as const, value: undefined }));
    const signer: SigningBackend = {
      presign,
      sign: async () => ({ ok: true, value: undefined }),
      postsign: async () => ({
        ok: true,
        value: { name: "factura.pdf", folder: "Documentos", sizeBytes: 1 },
      }),
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
    const presign = vi.fn(async () => ({ ok: true as const, value: undefined }));
    const signer: SigningBackend = {
      presign,
      sign: async () => ({ ok: true, value: undefined }),
      postsign: async () => ({
        ok: true,
        value: { name: "factura.pdf", folder: "Documentos", sizeBytes: 1 },
      }),
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
    const presign = vi.fn(async () => ({ ok: true as const, value: undefined }));
    const signer: SigningBackend = {
      presign,
      sign: async () => ({ ok: true, value: undefined }),
      postsign: async () => ({
        ok: true,
        value: { name: "factura.pdf", folder: "Documentos", sizeBytes: 1 },
      }),
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
