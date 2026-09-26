import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "./App";
import {
  aCertificate,
  aDestination,
  document,
  openPdf,
  pdfsOf,
  renderApp,
  row,
} from "./App.testSupport";
import { inMemoryExternalDestinationOpener } from "./desktop/externalDestination";
import { inMemoryDocumentDrops } from "./documents/drops";
import { inMemoryDocumentPicker } from "./documents/picker";
import { inMemoryRecents } from "./documents/recents";
import type { PreferencesStore } from "./preferences/preferences";
import type { Certificate } from "./signing/certificate";
import { emptyCertificateStore } from "./signing/certificate";
import { unavailableOpener } from "./signing/destination";
import { unavailableSigningBackend } from "./signing/flow";
import { emptyRubricPicker } from "./signing/rubric";
import { unavailableStampComposer } from "./signing/stampPreview";
import { renderWithCatalog } from "./testing/render";
import { inMemoryVersionCheck } from "./updates/newVersion";
import { unavailablePdfSource } from "./viewer/source";

// Grada A: el menú, los diálogos que abre y Privacidad, sobre la aplicación entera.
describe("App", () => {
  it("changes nothing when the dialog is closed without choosing", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")], pdfsOf({ "factura.pdf": 3 }));
    await openPdf(user);
    await screen.findByRole("region", { name: "Panel de firma" });

    // El selector en memoria se agota tras el primero, y a partir de ahí se
    // comporta como una cancelación (ID-73).
    await openPdf(user);

    const panel = screen.getByRole("region", { name: "Panel de firma" });
    expect(within(panel).getByText("factura.pdf")).toBeInTheDocument();
    const tabs = screen.getByRole("navigation", { name: "Documentos abiertos" });
    expect(within(tabs).getAllByRole("tab")).toHaveLength(1);
  });

  it("opens Preferences from the menu, replacing the tabs and the viewer", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents([row("a.pdf")]));
    await screen.findByText("a.pdf");

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));

    expect(await screen.findByRole("region", { name: "Preferencias" })).toBeInTheDocument();
    expect(
      screen.queryByRole("navigation", { name: "Documentos abiertos" }),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(screen.queryByRole("region", { name: "Preferencias" })).not.toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Documentos abiertos" })).toBeInTheDocument();
  });

  it("closes Preferences with Escape", async () => {
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));

    expect(screen.getByRole("region", { name: "Preferencias" })).toBeInTheDocument();

    await user.keyboard("{Escape}");

    expect(screen.queryByRole("region", { name: "Preferencias" })).not.toBeInTheDocument();
  });

  it("keeps the header and menu reachable while Preferences is open", async () => {
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));

    expect(screen.getByRole("region", { name: "Preferencias" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    expect(screen.getByRole("menu")).toBeInTheDocument();

    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Preferencias" })).toBeInTheDocument();
  });

  it("goes from Preferences to Estado de rFirma through the menu, and back", async () => {
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    expect(screen.getByRole("region", { name: "Preferencias" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Estado de rFirma" }));
    expect(screen.queryByRole("region", { name: "Preferencias" })).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Estado de rFirma" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    expect(screen.queryByRole("heading", { name: "Estado de rFirma" })).not.toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Preferencias" })).toBeInTheDocument();
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
    await user.click(await screen.findByRole("tab", { name: "Certificados" }));
    const certificates = screen.getByRole("tabpanel", { name: "Certificados" });
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
    await user.click(await screen.findByRole("tab", { name: "Certificados" }));
    const certificates = screen.getByRole("tabpanel", { name: "Certificados" });
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

  it("opens Estado de rFirma from the menu, and Cerrar closes it", async () => {
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Estado de rFirma" }));

    expect(screen.getByRole("heading", { name: "Estado de rFirma" })).toBeInTheDocument();
    expect(
      screen.queryByRole("navigation", { name: "Documentos abiertos" }),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(screen.queryByRole("heading", { name: "Estado de rFirma" })).not.toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Documentos abiertos" })).toBeInTheDocument();
  });

  it("closes Estado de rFirma with Escape", async () => {
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Estado de rFirma" }));

    expect(screen.getByRole("heading", { name: "Estado de rFirma" })).toBeInTheDocument();

    await user.keyboard("{Escape}");

    expect(screen.queryByRole("heading", { name: "Estado de rFirma" })).not.toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Documentos abiertos" })).toBeInTheDocument();
  });

  it("keeps the header and menu reachable while Estado de rFirma is open", async () => {
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Estado de rFirma" }));

    expect(screen.getByRole("heading", { name: "Estado de rFirma" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    expect(screen.getByRole("menu")).toBeInTheDocument();

    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Estado de rFirma" })).toBeInTheDocument();
  });

  it("opens About from the menu", async () => {
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Acerca de rFirma" }));

    expect(screen.getByText(/Proyecto independiente/)).toBeInTheDocument();
  });

  it("opens Comments and help from the menu", async () => {
    const user = userEvent.setup();
    const destinations = inMemoryExternalDestinationOpener();
    renderApp(
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      destinations,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Comentarios y ayuda" }));

    expect(destinations.opened).toEqual(["discussions"]);
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("empties the recents when Remember my activity is turned off", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents([row("a.pdf")]));
    await screen.findByText("a.pdf");

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    await user.click(await screen.findByRole("switch", { name: /Recordar mi actividad/ }));
    await user.click(screen.getByRole("button", { name: "Borrar y apagar" }));
    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    await waitFor(() => expect(screen.queryByText("a.pdf")).not.toBeInTheDocument());
    expect(screen.queryByRole("region", { name: "Recientes" })).not.toBeInTheDocument();
  });

  /**
   * Los recientes se vacían **aunque el borrado del disco falle** —lo que promete
   * el rótulo es que dejen de estar— y el fallo se cuenta en Privacidad, que es
   * el otro `catch {}` vacío que el ID-70 llena.
   */
  it("empties the recents and says they are still saved when the disk refuses", async () => {
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
        setupWizardSeen: false,
        consentCountdown: true,
        honourAutomaticSelection: false,
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
    expect(screen.getByRole("group", { name: "Privacidad" })).toContainElement(notice);
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

    await openPdf(user);
    await screen.findByRole("tab", { name: /factura\.pdf/ });
    await user.click(screen.getByRole("button", { name: "Abrir un PDF" }));

    expect(screen.getByRole("banner")).toHaveTextContent("Sin firmar");
    expect(screen.getByRole("menuitem", { name: "Abrir un PDF…" })).toBeInTheDocument();
    expect(screen.queryByText("Recientes")).not.toBeInTheDocument();
  });
});
