import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";
import {
  aCertificate,
  aDestination,
  document,
  failingCertificateStore,
  openPdf,
  pdfsOf,
  renderApp,
} from "./App.testSupport";
import { inMemoryRecents } from "./documents/recents";
import type { PreferencesStore } from "./preferences/preferences";
import type { RubricPicker } from "./signing/rubric";
import { DEFAULT_VISIBLE_SIGNATURE } from "./signing/visibleSignature";
import { aMainWindowDoubles } from "./testing/mainWindowDoubles";
import { renderWithCatalog } from "./testing/render";
import { unavailablePdfSource } from "./viewer/source";

// Grada A: la aplicación entera, con los cinco puertos en memoria.
describe("App", () => {
  /**
   * El tema es lo único de los ajustes que se pinta **fuera** del árbol de
   * React: los tokens de color cuelgan de `<html>`.
   */
  it("puts the remembered theme on the document as soon as the settings are read", async () => {
    renderApp({
      recents: inMemoryRecents(),
      documents: [],
      pdfs: unavailablePdfSource(),
      settings: { theme: "dark" },
    });

    await waitFor(() =>
      expect(globalThis.document.documentElement).toHaveAttribute("data-theme", "dark"),
    );
  });

  it("leaves the theme to the desktop when nothing was chosen", async () => {
    globalThis.document.documentElement.setAttribute("data-theme", "dark");
    renderApp({
      recents: inMemoryRecents(),
      documents: [],
      pdfs: unavailablePdfSource(),
      settings: { theme: "system" },
    });

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
    renderApp({
      recents: inMemoryRecents(),
      documents: [document("factura.pdf")],
      pdfs: pdfsOf({ "factura.pdf": 2 }),
      settings: {},
      certificates: failingCertificateStore(0, [{ ...aCertificate, remembered: true }]),
      rubrics,
    });

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    // La firma visible arranca apagada (#974): hay que encenderla para ver la
    // rúbrica que ya se adoptó.
    await user.click(within(panel).getByRole("switch", { name: "Firma visible" }));

    expect(await within(panel).findByRole("img", { name: "Tu rúbrica" })).toBeInTheDocument();
  });

  // Encender el interruptor enseña el modelo y la rúbrica recordados; no los siembra.
  it("starts a new document with the model and rubric flag a previous session left", async () => {
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents(),
      documents: [document("factura.pdf")],
      pdfs: pdfsOf({ "factura.pdf": 2 }),
      settings: {},
      certificates: failingCertificateStore(0, [{ ...aCertificate, remembered: true }]),
      initialSignature: { enabled: false, withRubric: true, content: { model: "rubricOnly" } },
    });

    await openPdf(user);
    const panel = await screen.findByRole("region", { name: "Panel de firma" });
    await user.click(within(panel).getByRole("switch", { name: "Firma visible" }));

    expect(within(panel).getByRole("radio", { name: "Solo rúbrica" })).toBeChecked();
    expect(within(panel).getByRole("switch", { name: "Con rúbrica" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
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
        destinationMode: "next_to_the_original",
        offersOriginalFolder: false,
        rememberVisibleSignature: true,
        rememberActivity: true,
        notifyNewVersion: true,
        setupWizardSeen: false,
        consentCountdown: true,
        honourAutomaticSelection: false,
        allowSha1: false,
      }),
      save: refused,
      forgetActivity: async () => {},
      chooseFolder: async () => null,
    };
    renderWithCatalog(
      <App
        ports={aMainWindowDoubles({
          preferences,
          destinations: aDestination(),
        })}
        initialSignature={DEFAULT_VISIBLE_SIGNATURE}
        version="0.1.0"
        menuAnchor="header"
      />,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    await user.click(await screen.findByRole("tab", { name: "Firma" }));
    const remember = await screen.findByRole("switch", {
      name: /Recordar la firma visible/,
    });
    await user.click(remember);

    // Se intentó guardar —así que el clic sí llegó— y aun así el interruptor
    // vuelve a estar como estaba, y ahora además se dice, en su sección.
    await waitFor(() => expect(refused).toHaveBeenCalledOnce());
    expect(remember).toHaveAttribute("aria-checked", "true");
    const notice = await screen.findByRole("alert");
    expect(notice).toHaveTextContent("Algo ha fallado");
    expect(screen.getByRole("tabpanel", { name: "Firma" })).toContainElement(notice);
    expect(screen.getByText("no se deja escribir")).toBeInTheDocument();
  });

  it("opens a document from the + menu without putting its badge in the header", async () => {
    const user = userEvent.setup();
    renderApp({ recents: inMemoryRecents(), documents: [document("factura.pdf")] });

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
    renderApp({
      recents: inMemoryRecents(),
      documents: [document("factura.pdf")],
      pdfs: pdfsOf({ "factura.pdf": 7 }),
    });

    await openPdf(user);

    await screen.findByRole("region", { name: "Panel de firma" });
    expect(screen.getByRole("tab", { name: "factura.pdf", selected: true })).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Arrastra un PDF o pulsa para abrirlo" }),
    ).not.toBeInTheDocument();
  });

  it("names the error of a PDF it cannot read instead of leaving an empty viewer", async () => {
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents(),
      documents: [document("corrupto.pdf")],
      pdfs: pdfsOf({}),
    });

    await openPdf(user);

    expect(await screen.findByRole("alert")).toHaveTextContent("No se ha podido leer el documento");
  });

  it("repaints a document when its tab is chosen again, one after another", async () => {
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents(),
      documents: [document("primero.pdf"), document("segundo.pdf")],
      pdfs: pdfsOf({ "primero.pdf": 2, "segundo.pdf": 5 }),
    });
    await openPdf(user);
    await screen.findByRole("region", { name: "Panel de firma" });

    await openPdf(user);
    await screen.findByRole("region", { name: "Panel de firma" });
    await screen.findByRole("tab", { name: "segundo.pdf", selected: true });

    await user.click(screen.getByRole("tab", { name: "primero.pdf" }));

    await waitFor(() =>
      expect(screen.getByRole("tab", { name: "primero.pdf", selected: true })).toBeInTheDocument(),
    );
  });
});
