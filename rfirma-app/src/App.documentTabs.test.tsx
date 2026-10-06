import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { inMemoryRecents } from "./documents/recents";
import { document, openPdf, pdfsOf, renderApp, row } from "./testing/harness";

const now = () => Math.floor(Date.now() / 1000);
const DAY = 86_400;

function panelShows(name: string) {
  screen.getByRole("region", { name: "Panel de firma" });
  return screen.queryByRole("tab", { name, selected: true }) !== null;
}

async function openRecentlyOpenedMenu(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole("button", { name: "Abiertos recientemente" }));
  return screen.getByRole("menu");
}

// Grada A: la tira de pestañas y el botón partido de abrir, sobre la aplicación entera.
describe("App, con varios documentos abiertos", () => {
  it("keeps several documents open and changes document when the tab changes", async () => {
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents(),
      documents: [document("primero.pdf"), document("segundo.pdf")],
      pdfs: pdfsOf({ "primero.pdf": 2, "segundo.pdf": 5 }),
    });
    await openPdf(user);
    await openPdf(user);
    await waitFor(() => expect(panelShows("segundo.pdf")).toBe(true));

    await user.click(screen.getByRole("tab", { name: "primero.pdf" }));

    await waitFor(() => expect(panelShows("primero.pdf")).toBe(true));
    expect(screen.getByRole("tab", { name: "primero.pdf", selected: true })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "segundo.pdf", selected: false })).toBeInTheDocument();
  });

  it("underlines only the active tab, and leaves the rest muted", async () => {
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents(),
      documents: [document("primero.pdf"), document("segundo.pdf")],
      pdfs: pdfsOf({ "primero.pdf": 2, "segundo.pdf": 5 }),
    });
    await openPdf(user);
    await openPdf(user);
    await screen.findByRole("tab", { name: "segundo.pdf", selected: true });

    const active = screen.getByRole("tab", { name: "segundo.pdf" }).closest(".document-tab");
    const muted = screen.getByRole("tab", { name: "primero.pdf" }).closest(".document-tab");

    expect(active).toHaveClass("document-tab--active");
    expect(muted).not.toHaveClass("document-tab--active");
  });

  it("moves focus between the tabs with the arrow keys, and only the active one is tabbable", async () => {
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents(),
      documents: [document("primero.pdf"), document("segundo.pdf")],
      pdfs: pdfsOf({ "primero.pdf": 2, "segundo.pdf": 5 }),
    });
    await openPdf(user);
    await openPdf(user);
    const active = await screen.findByRole("tab", { name: "segundo.pdf", selected: true });
    const other = screen.getByRole("tab", { name: "primero.pdf" });
    expect(active).toHaveAttribute("tabindex", "0");
    expect(other).toHaveAttribute("tabindex", "-1");

    active.focus();
    await user.keyboard("{ArrowLeft}");
    expect(other).toHaveFocus();

    await user.keyboard("{ArrowRight}");
    expect(active).toHaveFocus();
    expect(active).toHaveAttribute("aria-selected", "true");
  });

  it("names each tab after its file, whole in its tooltip, with a check when it is signed", async () => {
    const user = userEvent.setup();
    const long = `contrato-de-arrendamiento-${"largo-".repeat(8)}.pdf`;
    renderApp({
      recents: inMemoryRecents(),
      documents: [document(long, { badge: "Signed" })],
      pdfs: pdfsOf({ [long]: 1 }),
    });

    await openPdf(user);

    const tab = await screen.findByRole("tab", { selected: true });
    expect(tab).toHaveTextContent(long);
    expect(tab.closest("[title]")).toHaveAttribute("title", long);
    expect(within(tab).getByRole("img", { name: "Firmado" })).toBeInTheDocument();
  });

  it("closes a tab with its cross and puts the neighbouring document in front", async () => {
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents(),
      documents: [document("primero.pdf"), document("segundo.pdf")],
      pdfs: pdfsOf({ "primero.pdf": 2, "segundo.pdf": 5 }),
    });
    await openPdf(user);
    await openPdf(user);
    await screen.findByRole("tab", { name: "segundo.pdf", selected: true });

    await user.click(screen.getByRole("button", { name: "Cerrar segundo.pdf" }));

    expect(screen.queryByRole("tab", { name: "segundo.pdf" })).not.toBeInTheDocument();
    await waitFor(() => expect(panelShows("primero.pdf")).toBe(true));
  });

  it("goes back to the drop zone once the last tab is closed", async () => {
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents(),
      documents: [document("factura.pdf")],
      pdfs: pdfsOf({ "factura.pdf": 2 }),
    });
    await openPdf(user);
    await screen.findByRole("region", { name: "Panel de firma" });

    await user.click(screen.getByRole("button", { name: "Cerrar factura.pdf" }));

    expect(screen.queryByRole("tab")).not.toBeInTheDocument();
    expect(
      await screen.findByRole("button", { name: /Arrastra un PDF o pulsa para abrirlo/ }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Panel de firma" })).not.toBeInTheDocument();
  });
});

describe("App, la flecha de «Abiertos recientemente»", () => {
  it("offers opening a PDF directly, without a menu", async () => {
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByRole("button", { name: "Abrir PDF…" }));

    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("has no arrow when there are no recents", async () => {
    renderApp();

    await screen.findByRole("button", { name: "Abrir PDF…" });
    expect(
      screen.queryByRole("button", { name: "Abiertos recientemente" }),
    ).not.toBeInTheDocument();
  });

  it("has no arrow with Recordar mi actividad apagado, even with recents already saved", async () => {
    renderApp({
      recents: inMemoryRecents([row("a.pdf")]),
      documents: [],
      pdfs: pdfsOf({}),
      settings: { rememberActivity: false },
    });

    await screen.findByRole("button", { name: "Abrir PDF…" });
    expect(
      screen.queryByRole("button", { name: "Abiertos recientemente" }),
    ).not.toBeInTheDocument();
  });

  it("lists the recents with their date and their check", async () => {
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents([
        row("hoy.pdf", { lastUsed: now() }),
        row("ayer.pdf", { lastUsed: now() - DAY, badge: "Signed" }),
      ]),
    });
    await screen.findByRole("region", { name: "Abiertos recientemente" });

    const menu = await openRecentlyOpenedMenu(user);

    expect(within(menu).getByText("Abiertos recientemente")).toBeInTheDocument();
    expect(within(menu).getByRole("menuitem", { name: /^hoy\.pdf/ })).toHaveTextContent("hoy");
    const signed = within(menu).getByRole("menuitem", { name: /^ayer\.pdf/ });
    expect(signed).toHaveTextContent("ayer");
    expect(within(signed).getByRole("img", { name: "Firmado" })).toBeInTheDocument();
    expect(within(menu).getByRole("menuitem", { name: "Vaciar la lista" })).toBeInTheDocument();
  });

  it("opens a recent in a new tab", async () => {
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents([row("a.pdf")]),
      documents: [],
      pdfs: pdfsOf({ "a.pdf": 3 }),
    });
    await screen.findByRole("region", { name: "Abiertos recientemente" });

    const menu = await openRecentlyOpenedMenu(user);
    await user.click(within(menu).getByRole("menuitem", { name: /^a\.pdf/ }));

    expect(await screen.findByRole("tab", { name: "a.pdf", selected: true })).toBeInTheDocument();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("says Abierto for a recent that already has a tab, and takes the user to it", async () => {
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents(),
      documents: [document("primero.pdf"), document("segundo.pdf")],
      pdfs: pdfsOf({ "primero.pdf": 2, "segundo.pdf": 5 }),
    });
    await openPdf(user);
    await openPdf(user);
    await screen.findByRole("tab", { name: "segundo.pdf", selected: true });

    const menu = await openRecentlyOpenedMenu(user);
    const open = within(menu).getByRole("menuitem", { name: /^primero\.pdf/ });
    expect(open).toHaveTextContent("Abierto");
    expect(open).toHaveAttribute("title", "Ir a su pestaña");
    await user.click(open);

    expect(screen.getAllByRole("tab")).toHaveLength(2);
    expect(screen.getByRole("tab", { name: "primero.pdf", selected: true })).toBeInTheDocument();
  });

  it("shows the folder under the name when it is known, and as its title", async () => {
    const user = userEvent.setup();
    renderApp({ recents: inMemoryRecents([row("hoy.pdf", { folder: "Documentos" })]) });
    await screen.findByRole("region", { name: "Abiertos recientemente" });

    const menu = await openRecentlyOpenedMenu(user);

    const item = within(menu).getByRole("menuitem", { name: /^hoy\.pdf/ });
    expect(item).toHaveTextContent("Documentos");
    expect(item).toHaveAttribute("title", "Documentos");
  });

  it("shows only the name and no title when the folder is unknown, under the portal", async () => {
    const user = userEvent.setup();
    renderApp({ recents: inMemoryRecents([row("hoy.pdf", { folder: null, lastUsed: now() })]) });
    await screen.findByRole("region", { name: "Abiertos recientemente" });

    const menu = await openRecentlyOpenedMenu(user);

    // El nombre accesible junta las dos líneas de la fila: si fuera exactamente
    // "hoy.pdf" + «hoy» (la fecha de hoy), no hay ninguna carpeta entre medias.
    const item = within(menu).getByRole("menuitem", { name: "hoy.pdfhoy" });
    expect(item).not.toHaveAttribute("title");
  });

  it("dims a recent that is no longer where it was, and does not open it", async () => {
    const user = userEvent.setup();
    renderApp({ recents: inMemoryRecents([row("usb.pdf", { available: false })]) });
    await screen.findByRole("region", { name: "Abiertos recientemente" });

    const menu = await openRecentlyOpenedMenu(user);

    const missing = within(menu).getByRole("menuitem", { name: /^usb\.pdf/ });
    expect(missing).toBeDisabled();
    expect(missing).toHaveTextContent("No se encuentra");
    expect(missing).toHaveAttribute("title", "No se encuentra");
  });

  it("says No se encuentra instead of the folder, even when the folder is known", async () => {
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents([row("usb.pdf", { available: false, folder: "Documentos" })]),
    });
    await screen.findByRole("region", { name: "Abiertos recientemente" });

    const menu = await openRecentlyOpenedMenu(user);

    const missing = within(menu).getByRole("menuitem", { name: /^usb\.pdf/ });
    expect(missing).toHaveTextContent("No se encuentra");
    expect(missing).not.toHaveTextContent("Documentos");
  });

  it("empties the recents from Vaciar la lista, and hides the arrow when none are left", async () => {
    const user = userEvent.setup();
    const recents = inMemoryRecents([row("a.pdf")]);
    renderApp({ recents });
    await screen.findByRole("region", { name: "Abiertos recientemente" });

    await user.click(
      within(await openRecentlyOpenedMenu(user)).getByRole("menuitem", { name: "Vaciar la lista" }),
    );

    await waitFor(() => expect(screen.queryByText("a.pdf")).not.toBeInTheDocument());
    await expect(recents.list()).resolves.toEqual([]);
    expect(
      screen.queryByRole("button", { name: "Abiertos recientemente" }),
    ).not.toBeInTheDocument();
  });

  it("closes with Escape", async () => {
    const user = userEvent.setup();
    renderApp({ recents: inMemoryRecents([row("a.pdf")]) });
    await openRecentlyOpenedMenu(user);

    await user.keyboard("{Escape}");

    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });
});

describe("App, sin documentos abiertos", () => {
  it("shows the drop zone and the recents in the middle of the viewer", async () => {
    renderApp({ recents: inMemoryRecents([row("a.pdf", { lastUsed: now() })]) });

    const viewer = screen.getByRole("region", { name: "Visor del documento" });
    expect(
      within(viewer).getByRole("button", { name: /Arrastra un PDF o pulsa para abrirlo/ }),
    ).toBeInTheDocument();
    const recents = await within(viewer).findByRole("region", { name: "Abiertos recientemente" });
    expect(within(recents).getByRole("button", { name: /^a\.pdf/ })).toHaveTextContent("hoy");
    expect(within(recents).getByRole("button", { name: "Vaciar la lista" })).toBeInTheDocument();
  });

  it("opens a recent from the empty viewer in a tab", async () => {
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents([row("a.pdf")]),
      documents: [],
      pdfs: pdfsOf({ "a.pdf": 3 }),
    });
    const recents = await screen.findByRole("region", { name: "Abiertos recientemente" });

    await user.click(within(recents).getByRole("button", { name: /^a\.pdf/ }));

    expect(await screen.findByRole("tab", { name: "a.pdf", selected: true })).toBeInTheDocument();
    expect(
      screen.queryByRole("region", { name: "Abiertos recientemente" }),
    ).not.toBeInTheDocument();
  });

  it("shows only the drop zone when there is nothing recent", async () => {
    renderApp();

    await screen.findByRole("button", { name: /Arrastra un PDF o pulsa para abrirlo/ });
    expect(
      screen.queryByRole("region", { name: "Abiertos recientemente" }),
    ).not.toBeInTheDocument();
  });

  it("shows only the drop zone with Recordar mi actividad apagado, even with recents already saved", async () => {
    renderApp({
      recents: inMemoryRecents([row("a.pdf")]),
      documents: [],
      pdfs: pdfsOf({}),
      settings: { rememberActivity: false },
    });

    await screen.findByRole("button", { name: /Arrastra un PDF o pulsa para abrirlo/ });
    expect(
      screen.queryByRole("region", { name: "Abiertos recientemente" }),
    ).not.toBeInTheDocument();
  });
});

function stubTabStripWidth() {
  const observed = new Map<Element, () => void>();
  class Stub {
    private readonly callback: () => void;
    constructor(callback: () => void) {
      this.callback = callback;
    }
    observe(element: Element) {
      observed.set(element, this.callback);
    }
    unobserve() {}
    disconnect() {}
  }
  vi.stubGlobal("ResizeObserver", Stub);
  return (width: number) => {
    const strip = screen.getByRole("navigation", { name: "Documentos abiertos" });
    Object.defineProperty(strip, "clientWidth", { value: width, configurable: true });
    act(() => observed.get(strip)?.());
  };
}

const FIVE = ["uno.pdf", "dos.pdf", "tres.pdf", "cuatro.pdf", "cinco.pdf"];

async function openFive(user: ReturnType<typeof userEvent.setup>) {
  renderApp({
    recents: inMemoryRecents(),
    documents: FIVE.map((name) => document(name)),
    pdfs: pdfsOf(Object.fromEntries(FIVE.map((name) => [name, 1]))),
  });
  for (const _ of FIVE) await openPdf(user);
  await screen.findByRole("tab", { name: "cinco.pdf", selected: true });
}

const tabNames = () => screen.getAllByRole("tab").map((tab) => tab.textContent);

describe("App, con más pestañas de las que caben", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("shows +N with the number of hidden tabs, and keeps the active one in sight", async () => {
    const resizeStrip = stubTabStripWidth();
    const user = userEvent.setup();
    await openFive(user);

    resizeStrip(780);

    expect(tabNames()).toEqual(["uno.pdf", "dos.pdf", "cinco.pdf"]);
    expect(screen.getByRole("button", { name: "Más pestañas" })).toHaveTextContent("+2");
  });

  it("changes the split when the window is resized", async () => {
    const resizeStrip = stubTabStripWidth();
    const user = userEvent.setup();
    await openFive(user);
    resizeStrip(780);

    resizeStrip(2000);

    expect(tabNames()).toEqual(FIVE);
    expect(screen.queryByRole("button", { name: "Más pestañas" })).not.toBeInTheDocument();
  });

  it("brings a hidden tab to the strip as the active one from the recents", async () => {
    const resizeStrip = stubTabStripWidth();
    const user = userEvent.setup();
    await openFive(user);
    resizeStrip(780);

    const menu = await openRecentlyOpenedMenu(user);
    await user.click(within(menu).getByRole("menuitem", { name: /^tres\.pdf/ }));

    expect(tabNames()).toEqual(["uno.pdf", "dos.pdf", "tres.pdf"]);
    expect(screen.getByRole("tab", { name: "tres.pdf", selected: true })).toBeInTheDocument();
  });

  it("opens the +N menu aligned to it, listing the hidden tabs in order with a check for the signed ones", async () => {
    const resizeStrip = stubTabStripWidth();
    const user = userEvent.setup();
    renderApp({
      recents: inMemoryRecents(),
      documents: [
        document("uno.pdf"),
        document("dos.pdf"),
        document("tres.pdf", { badge: "Signed" }),
        document("cuatro.pdf"),
        document("cinco.pdf"),
      ],
      pdfs: pdfsOf(Object.fromEntries(FIVE.map((name) => [name, 1]))),
    });
    for (const _ of FIVE) await openPdf(user);
    await screen.findByRole("tab", { name: "cinco.pdf", selected: true });
    resizeStrip(780);
    const more = screen.getByRole("button", { name: "Más pestañas" });

    await user.click(more);

    expect(more).toHaveAttribute("aria-expanded", "true");
    const menu = within(screen.getByRole("menu"));
    expect(menu.getAllByRole("menuitem").map((item) => item.textContent)).toEqual([
      "tres.pdf",
      "cuatro.pdf",
    ]);
    expect(
      within(menu.getByRole("menuitem", { name: /^tres\.pdf/ })).getByRole("img", {
        name: "Firmado",
      }),
    ).toBeInTheDocument();
    expect(
      within(menu.getByRole("menuitem", { name: "cuatro.pdf" })).queryByRole("img", {
        name: "Firmado",
      }),
    ).not.toBeInTheDocument();
  });

  it("activates a hidden tab from the +N menu, bringing it to the strip and letting another take its place", async () => {
    const resizeStrip = stubTabStripWidth();
    const user = userEvent.setup();
    await openFive(user);
    resizeStrip(780);
    await user.click(screen.getByRole("button", { name: "Más pestañas" }));

    await user.click(screen.getByRole("menuitem", { name: "tres.pdf" }));

    expect(tabNames()).toEqual(["uno.pdf", "dos.pdf", "tres.pdf"]);
    expect(screen.getByRole("tab", { name: "tres.pdf", selected: true })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Más pestañas" }));
    expect(
      within(screen.getByRole("menu"))
        .getAllByRole("menuitem")
        .map((i) => i.textContent),
    ).toEqual(["cuatro.pdf", "cinco.pdf"]);
  });

  it("closes the +N menu with Escape and by clicking outside", async () => {
    const resizeStrip = stubTabStripWidth();
    const user = userEvent.setup();
    await openFive(user);
    resizeStrip(780);
    await user.click(screen.getByRole("button", { name: "Más pestañas" }));
    expect(screen.getByRole("menu")).toBeInTheDocument();

    await user.keyboard("{Escape}");

    expect(screen.queryByRole("menu")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Más pestañas" }));
    await user.click(screen.getByRole("tab", { name: "uno.pdf" }));

    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });
});
