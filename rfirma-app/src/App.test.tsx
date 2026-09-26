import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";
import {
  aCertificate,
  aDestination,
  destinationOfferingSingleChoice,
  document,
  failingCertificateStore,
  openPdf,
  pdfsOf,
  renderApp,
} from "./App.testSupport";
import { inMemoryDocumentDrops } from "./documents/drops";
import { inMemoryDocumentPicker } from "./documents/picker";
import { inMemoryRecents } from "./documents/recents";
import type { PreferencesStore } from "./preferences/preferences";
import type { Certificate } from "./signing/certificate";
import { emptyCertificateStore } from "./signing/certificate";
import { unavailableOpener } from "./signing/destination";
import { unavailableSigningBackend } from "./signing/flow";
import { emptyRubricPicker, type RubricPicker } from "./signing/rubric";
import { unavailableStampComposer } from "./signing/stampPreview";
import { renderWithCatalog } from "./testing/render";
import { inMemoryVersionCheck } from "./updates/newVersion";
import { unavailablePdfSource } from "./viewer/source";

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
      // `remembered` porque sin él no hay preselección, ni con uno solo.
      failingCertificateStore(0, [{ ...aCertificate, remembered: true }]),
      rubrics,
    );

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    // La firma visible arranca apagada (#974): hay que encenderla para ver la
    // rúbrica que ya se adoptó.
    await user.click(
      within(panel).getByRole("switch", { name: /Estampar un recuadro de firma en el documento/ }),
    );

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
        setupWizardSeen: false,
        consentCountdown: true,
        honourAutomaticSelection: false,
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
    await user.click(await screen.findByRole("tab", { name: "Firma" }));
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
    expect(screen.getByRole("tabpanel", { name: "Firma" })).toContainElement(notice);
    expect(screen.getByText("no se deja escribir")).toBeInTheDocument();
  });

  it("opens a document from the + menu without putting its badge in the header", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")]);

    await openPdf(user);

    expect(await screen.findByText("factura.pdf")).toBeInTheDocument();
    expect(screen.getByRole("banner")).not.toHaveTextContent("Sin firmar");
  });

  /**
   * El recorrido entero del #82, contado por lo que se ve y no por las órdenes
   * que se llamaron (TD-15): se elige un PDF y queda pintado, con su nombre y
   * sus páginas en el panel, y en su pestaña.
   */
  it("paints the chosen document in the viewer and opens it in a tab", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")], pdfsOf({ "factura.pdf": 7 }));

    await openPdf(user);

    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    expect(within(panel).getByText("factura.pdf")).toBeInTheDocument();
    expect(within(panel).getByText(/^7 páginas/)).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "factura.pdf", selected: true })).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Arrastra un PDF o pulsa para abrirlo" }),
    ).not.toBeInTheDocument();
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

    await openPdf(user);
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
    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const retry = await within(panel).findByRole("button", { name: "Volver a buscar" });

    await user.click(retry);

    // Sin preselección tampoco tras resolverse el fallo: el desplegable trae
    // el certificado, pero sigue sin elegir ninguno.
    expect(await within(panel).findByRole("combobox", { name: "Certificado" })).toHaveTextContent(
      "Elegir certificado",
    );
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
    const other: Certificate = {
      ...aCertificate,
      id: "otra",
      holderName: "Grace Hopper Murray",
      givenName: "Grace",
      surname: "Hopper Murray",
    };
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      { list: async () => [aCertificate, other] },
    );
    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const trigger = await within(panel).findByRole("combobox", { name: "Certificado" });

    expect(trigger).toHaveTextContent("Elegir certificado");
    expect(within(panel).queryByRole("button", { name: /Firmar/ })).not.toBeInTheDocument();

    await user.click(trigger);
    // La lista vive en un portal, fuera de `panel` (ID-308): se busca en todo
    // el documento, no dentro del panel.
    const rows = screen.getAllByRole("option");
    const second = rows[1];
    if (second === undefined) throw new Error("la lista tenia que traer dos filas");
    await user.click(second);

    // La firma visible arranca apagada (#974): con certificado elegido y sin
    // encenderla, firmar ya está permitido.
    expect(within(panel).getByRole("button", { name: "Firmar como Grace Hopper" })).toBeEnabled();
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
      givenName: "Grace",
      surname: "Hopper Murray",
      remembered: true,
    };
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      { list: async () => [aCertificate, used] },
    );

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });

    // La firma visible arranca apagada (#974): con certificado elegido y sin
    // encenderla, firmar ya está permitido.
    expect(
      await within(panel).findByRole("button", { name: "Firmar como Grace Hopper" }),
    ).toBeEnabled();
  });

  /**
   * Y el recordado que ya no está —tarjeta fuera, perfil borrado— deja el panel
   * en «Sin certificado» **sin ruido**: no viene marcada ninguna fila, y eso no
   * es un error que contar (ADR-0010).
   */
  it("falls back to no certificate when the remembered one is gone, without an error", async () => {
    const user = userEvent.setup();
    const other: Certificate = {
      ...aCertificate,
      id: "otra",
      holderName: "Grace Hopper Murray",
      givenName: "Grace",
      surname: "Hopper Murray",
    };
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      { list: async () => [aCertificate, other] },
    );

    await openPdf(user);
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

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const trigger = await within(panel).findByRole("combobox", { name: "Certificado" });

    expect(trigger).toHaveTextContent("Elegir certificado");
  });

  /** El único caso que quedaba de «con uno solo se elige solo» —el
   * utilizable— también desaparece: nunca se elige por su cuenta. */
  it("does not preselect the sole certificate even when it can be used", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      { list: async () => [aCertificate] },
    );

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const trigger = await within(panel).findByRole("combobox", { name: "Certificado" });

    expect(trigger).toHaveTextContent("Elegir certificado");
    expect(within(panel).queryByRole("button", { name: /Firmar/ })).not.toBeInTheDocument();
  });

  /** «Con uno solo se elige solo» ya no tiene excepciones que gane: si ese
   * único no sirve, el desplegable sigue arrancando sin elección. */
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

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    const trigger = await within(panel).findByRole("combobox", { name: "Certificado" });

    expect(trigger).toHaveTextContent("Elegir certificado");
  });

  it("names the error of a PDF it cannot read instead of leaving an empty viewer", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("corrupto.pdf")], pdfsOf({}));

    await openPdf(user);

    expect(await screen.findByRole("alert")).toHaveTextContent("No hemos podido leer el documento");
  });

  it("repaints a document when its tab is chosen again, one after another", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("primero.pdf"), document("segundo.pdf")],
      pdfsOf({ "primero.pdf": 2, "segundo.pdf": 5 }),
    );
    await openPdf(user);
    await screen.findByRole("region", { name: "Panel de firma" });

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await waitFor(() => expect(within(panel).getByText("segundo.pdf")).toBeInTheDocument());

    await user.click(screen.getByRole("tab", { name: "primero.pdf" }));

    await waitFor(() => expect(within(panel).getByText("primero.pdf")).toBeInTheDocument());
    expect(within(panel).getByText(/^2 páginas/)).toBeInTheDocument();
  });

  /**
   * #974: la firma visible arranca apagada y firmar no la necesita; encenderla
   * es lo único que coloca el recuadro, sobre la página que se está mirando.
   */
  it("signs without a visible signature by default, and turning it on places the box", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 3 }),
      {},
      { list: async () => [{ ...aCertificate, remembered: true }] },
    );
    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await within(panel).findByRole("button", { name: "Firmar como Ada Lovelace" });

    const toggle = within(panel).getByRole("switch", {
      name: /Estampar un recuadro de firma en el documento/,
    });
    expect(toggle).toHaveAttribute("aria-checked", "false");
    expect(within(panel).getByRole("button", { name: "Firmar como Ada Lovelace" })).toBeEnabled();
    expect(
      screen.queryByRole("application", { name: "Recuadro de la firma visible" }),
    ).not.toBeInTheDocument();

    await user.click(toggle);

    expect(
      await screen.findByRole("application", { name: "Recuadro de la firma visible" }),
    ).toBeInTheDocument();
  });

  it("brings back the box where it was when the switch goes off and on again", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 3 }),
      {},
      { list: async () => [{ ...aCertificate, remembered: true }] },
    );
    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await within(panel).findByRole("button", { name: "Firmar como Ada Lovelace" });
    const toggle = within(panel).getByRole("switch", {
      name: /Estampar un recuadro de firma en el documento/,
    });
    const box = () => screen.queryByRole("application", { name: "Recuadro de la firma visible" });

    await user.click(toggle);
    expect(await within(panel).findByText("Página 1")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Página siguiente" }));
    await user.click(screen.getByRole("button", { name: "Página siguiente" }));
    await user.click(toggle);
    await user.click(toggle);

    expect(toggle).toHaveAttribute("aria-checked", "true");
    expect(within(panel).getByText("Página 1")).toBeInTheDocument();
    expect(within(panel).queryByText("Página 3")).not.toBeInTheDocument();
    expect(box()).not.toBeInTheDocument();
  });

  /**
   * #977: «Cambiar» abre el diálogo de guardar y fija carpeta y nombre para
   * esta firma **sin tocar** la preferencia de carpeta —lo que se guardaría en
   * disco al pasar por Preferencias—.
   */
  it("changes the folder and name shown in the footer for this signature, without touching the destination preference", async () => {
    const user = userEvent.setup();
    const destinations = destinationOfferingSingleChoice(
      { folder: "Documentos", name: "factura-firmado.pdf", writable: true },
      { id: "single-42", folder: "Escritorio", name: "factura-firmado-2.pdf", writable: true },
    );
    const { preferences } = renderApp(
      inMemoryRecents(),
      [document("factura.pdf")],
      pdfsOf({ "factura.pdf": 2 }),
      {},
      {},
      emptyRubricPicker(),
      unavailableSigningBackend(),
      null,
      undefined,
      inMemoryVersionCheck(),
      undefined,
      undefined,
      destinations,
    );

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });

    expect(await within(panel).findByText("…/Documentos/")).toBeInTheDocument();

    await user.click(within(panel).getByRole("button", { name: "Cambiar" }));

    expect(await within(panel).findByText("…/Escritorio/")).toBeInTheDocument();
    expect(within(panel).getByText(/factura-firmado-2\.pdf/)).toBeInTheDocument();
    await expect(preferences.read()).resolves.toMatchObject({ destination: "Documentos" });
  });
});
