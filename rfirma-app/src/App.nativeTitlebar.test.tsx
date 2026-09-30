import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { document, pdfsOf, renderApp } from "./App.testSupport";
import {
  type ExternalDestinationOpener,
  inMemoryExternalDestinationOpener,
  unavailableExternalDestinationOpener,
} from "./desktop/externalDestination";
import { inMemoryDocumentDrops } from "./documents/drops";
import { inMemoryRecents } from "./documents/recents";
import { openTab } from "./preferences/testSupport";
import { inMemoryNativeTitlebar, type TitlebarAction } from "./shell/nativeTitlebar";
import { emptyCertificateStore } from "./signing/certificate";
import { unavailableSigningBackend } from "./signing/flow";
import { emptyRubricPicker } from "./signing/rubric";
import { memoryStatus, type SignalRow, type StatusPort } from "./status/status";
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
  status = memoryStatus() as StatusPort,
  externalDestinations = unavailableExternalDestinationOpener() as ExternalDestinationOpener,
} = {}) {
  const titlebar = inMemoryNativeTitlebar();
  renderApp(
    inMemoryRecents(),
    pdfNames.map((name) => document(name)),
    pdfNames.length === 0
      ? unavailablePdfSource()
      : pdfsOf(Object.fromEntries(pdfNames.map((name) => [name, 1]))),
    {},
    emptyCertificateStore(),
    emptyRubricPicker(),
    unavailableSigningBackend(),
    null,
    inMemoryDocumentDrops(null),
    inMemoryVersionCheck(),
    externalDestinations,
    status,
    undefined,
    undefined,
    titlebar,
  );
  const press = (action: TitlebarAction) => act(() => titlebar.press(action));
  return { titlebar, press };
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
});
