import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import {
  type ExternalDestinationOpener,
  inMemoryExternalDestinationOpener,
  unavailableExternalDestinationOpener,
} from "./desktop/externalDestination";
import { inMemoryDocumentDrops } from "./documents/drops";
import { inMemoryRecents, type RecentDocument, type RecentsStore } from "./documents/recents";
import { openTab } from "./preferences/testing/harness";
import { inMemoryNativeTitlebar, type TitlebarActionName } from "./shell/nativeTitlebar";
import { emptyCertificateStore } from "./signing/certificate";
import { unavailableSigningBackend } from "./signing/flow";
import { emptyRubricPicker } from "./signing/rubric";
import { memoryStatus, type SignalRow, type StatusPort } from "./status/status";
import { document, pdfsOf, renderApp, row } from "./testing/harness";
import { inMemoryVersionCheck } from "./updates/newVersion";
import { unavailablePdfSource } from "./viewer/source";

const SPANISH_LABELS = {
  open: "Abrir PDF…",
  openTooltip: "Ctrl+O",
  warning: "Estado de rFirma: requiere atención",
  menu: "Menú",
  status: "Estado de rFirma",
  preferences: "Preferencias…",
  feedback: "Comentarios y ayuda",
  about: "Acerca de rFirma",
  recents: "Abiertos recientemente",
  clearRecents: "Vaciar la lista",
  notFound: "No se encuentra",
};

const SITES_UNCONFIGURED: SignalRow[] = [
  {
    signal: "siteSignature",
    value: "",
    verdict: "attention",
    action: null,
    detail: null,
    candidates: null,
    restartFirefoxNotice: false,
  },
];

function renderOnLinux({
  pdfNames = [] as string[],
  recentRows = [] as RecentDocument[],
  status = memoryStatus() as StatusPort,
  externalDestinations = unavailableExternalDestinationOpener() as ExternalDestinationOpener,
  store = inMemoryRecents(recentRows) as RecentsStore,
} = {}) {
  const titlebar = inMemoryNativeTitlebar();
  renderApp({
    recents: store,
    documents: pdfNames.map((name) => document(name)),
    pdfs:
      pdfNames.length === 0
        ? unavailablePdfSource()
        : pdfsOf(Object.fromEntries(pdfNames.map((name) => [name, 1]))),
    settings: {},
    certificates: emptyCertificateStore(),
    rubrics: emptyRubricPicker(),
    signer: unavailableSigningBackend(),
    invoked: null,
    drops: inMemoryDocumentDrops(null),
    versions: inMemoryVersionCheck(),
    externalDestinations,
    status,
    titlebar,
  });
  const press = (action: TitlebarActionName) => act(() => titlebar.press({ action }));
  const pressRecent = (path: string) => act(() => titlebar.press({ action: "recent", path }));
  return { titlebar, press, pressRecent };
}

// Grada A: la aplicación entera en Linux, con el doble de la barra de título GTK.
describe("App with the native titlebar", () => {
  it("paints no header at all while there is no tab", () => {
    renderOnLinux();

    expect(screen.queryByRole("navigation", { name: "Documentos abiertos" })).toBeNull();
    expect(screen.queryByText("rFirma")).toBeNull();
    expect(screen.queryByRole("button", { name: "Menú" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Abrir PDF…" })).toBeNull();
  });

  it("keeps only the tabs in the strip once a document is open", async () => {
    const { press } = renderOnLinux({ pdfNames: ["factura.pdf"] });

    await press("open");

    const strip = await screen.findByRole("navigation", { name: "Documentos abiertos" });
    expect(within(strip).getByRole("tab", { name: "factura.pdf" })).toBeInTheDocument();
    expect(within(strip).queryByRole("button", { name: "Abrir PDF…" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Menú" })).toBeNull();
    expect(screen.queryByText("rFirma")).toBeNull();
  });

  it("drops the strip again in Preferences and in the status panel", async () => {
    const { press } = renderOnLinux({ pdfNames: ["factura.pdf"] });
    await press("open");
    await screen.findByRole("navigation", { name: "Documentos abiertos" });

    await press("preferences");
    expect(screen.queryByRole("navigation", { name: "Documentos abiertos" })).toBeNull();

    await press("status");
    expect(screen.queryByRole("navigation", { name: "Documentos abiertos" })).toBeNull();
  });

  it("sends the translated state at startup, with the open button showing", () => {
    const { titlebar } = renderOnLinux();

    expect(titlebar.latest).toEqual({
      openVisible: true,
      warningVisible: false,
      labels: SPANISH_LABELS,
      recents: [],
    });
  });

  it("hides the open button in the views without a document, and brings it back", async () => {
    const user = userEvent.setup();
    const { titlebar, press } = renderOnLinux();

    await press("preferences");
    expect(titlebar.latest?.openVisible).toBe(false);

    await user.click(screen.getByRole("button", { name: "Cerrar" }));
    expect(titlebar.latest?.openVisible).toBe(true);
  });

  it("shows the warning when something needs attention, and hides it inside the status panel", async () => {
    const { titlebar, press } = renderOnLinux({ status: memoryStatus(SITES_UNCONFIGURED) });

    await waitFor(() => expect(titlebar.latest?.warningVisible).toBe(true));

    await press("status");
    expect(titlebar.latest?.warningVisible).toBe(false);
    expect(titlebar.latest?.openVisible).toBe(false);
  });

  it("sends the labels again in the new language", async () => {
    const user = userEvent.setup();
    const { titlebar, press } = renderOnLinux();

    await press("preferences");
    await openTab(user, "Apariencia");
    await user.click(screen.getByRole("combobox", { name: "Idioma" }));
    await user.click(screen.getByRole("option", { name: "English" }));

    await waitFor(() => expect(titlebar.latest?.labels.open).toBe("Open PDF…"));
    expect(titlebar.latest?.labels.menu).toBe("Menu");
  });

  it("goes to the status panel on status", async () => {
    const { press } = renderOnLinux();

    await press("status");

    expect(screen.getByRole("heading", { name: "Estado de rFirma" })).toBeInTheDocument();
  });

  it("goes to Preferences on preferences", async () => {
    const { press } = renderOnLinux();

    await press("preferences");

    expect(await screen.findByRole("region", { name: "Preferencias" })).toBeInTheDocument();
  });

  it("opens Comments and help on feedback", async () => {
    const destinations = inMemoryExternalDestinationOpener();
    const { press } = renderOnLinux({ externalDestinations: destinations });

    await press("feedback");

    expect(destinations.opened).toEqual(["discussions"]);
  });

  it("opens About on about", async () => {
    const { press } = renderOnLinux();

    await press("about");

    expect(screen.getByText(/Proyecto independiente/)).toBeInTheDocument();
  });

  it("still opens a PDF with Ctrl+O", async () => {
    const user = userEvent.setup();
    renderOnLinux({ pdfNames: ["factura.pdf"] });

    await user.keyboard("{Control>}o{/Control}");

    expect(await screen.findByRole("tab", { name: "factura.pdf" })).toBeInTheDocument();
  });

  it("sends the recents with what each entry paints, and the labels for them", async () => {
    const { titlebar } = renderOnLinux({
      recentRows: [
        row("factura.pdf", { location: "~/Documentos", badge: "Signed" }),
        row("usb.pdf", { available: false }),
      ],
    });

    await waitFor(() => expect(titlebar.latest?.recents).toHaveLength(2));
    expect(titlebar.latest?.recents).toEqual([
      {
        path: "id-factura.pdf",
        name: "factura.pdf",
        location: "~/Documentos",
        signed: true,
        found: true,
      },
      { path: "id-usb.pdf", name: "usb.pdf", location: null, signed: false, found: false },
    ]);
    expect(titlebar.latest?.labels).toMatchObject({
      recents: "Abiertos recientemente",
      clearRecents: "Vaciar la lista",
      notFound: "No se encuentra",
    });
  });

  it("sends an empty list when there are no recents", () => {
    const { titlebar } = renderOnLinux();

    expect(titlebar.latest?.recents).toEqual([]);
  });

  it("sends an empty list when Recordar mi actividad is off", async () => {
    const user = userEvent.setup();
    const { titlebar, press } = renderOnLinux({ recentRows: [row("factura.pdf")] });
    await waitFor(() => expect(titlebar.latest?.recents).toHaveLength(1));

    await press("preferences");
    await user.click(await screen.findByRole("switch", { name: /Recordar mi actividad/ }));
    await user.click(screen.getByRole("button", { name: "Borrar y apagar" }));

    await waitFor(() => expect(titlebar.latest?.recents).toEqual([]));
  });

  it("opens a recent on recent, and goes to its tab if it is already open", async () => {
    const { titlebar, pressRecent } = renderOnLinux({
      pdfNames: ["factura.pdf"],
      recentRows: [row("factura.pdf")],
    });
    await waitFor(() => expect(titlebar.latest?.recents).toHaveLength(1));

    await pressRecent("id-factura.pdf");
    expect(await screen.findByRole("tab", { name: "factura.pdf" })).toBeInTheDocument();

    await pressRecent("id-factura.pdf");
    expect(screen.getAllByRole("tab", { name: "factura.pdf" })).toHaveLength(1);
  });

  it("never opens a recent that is not found", async () => {
    const { titlebar, pressRecent } = renderOnLinux({
      pdfNames: ["usb.pdf"],
      recentRows: [row("usb.pdf", { available: false })],
    });
    await waitFor(() => expect(titlebar.latest?.recents).toHaveLength(1));

    await pressRecent("id-usb.pdf");

    expect(screen.queryByRole("tab", { name: "usb.pdf" })).toBeNull();
  });

  it("sends a recent as not found once its file vanishes and the window regains focus", async () => {
    let found = true;
    const store = {
      ...inMemoryRecents(),
      list: async () => [row("usb.pdf", { available: found })],
    };
    const { titlebar } = renderOnLinux({ store });
    await waitFor(() => expect(titlebar.latest?.recents[0]?.found).toBe(true));

    found = false;
    await act(async () => {
      window.dispatchEvent(new Event("focus"));
    });
    await waitFor(() => expect(titlebar.latest?.recents[0]?.found).toBe(false));

    found = true;
    await act(async () => {
      window.dispatchEvent(new Event("focus"));
    });
    await waitFor(() => expect(titlebar.latest?.recents[0]?.found).toBe(true));
  });

  it("empties the recents on clearRecents", async () => {
    const { titlebar, press } = renderOnLinux({ recentRows: [row("factura.pdf")] });
    await waitFor(() => expect(titlebar.latest?.recents).toHaveLength(1));

    await press("clearRecents");

    await waitFor(() => expect(titlebar.latest?.recents).toEqual([]));
  });
});
