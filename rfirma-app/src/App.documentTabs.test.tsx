import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { document, openPdf, pdfsOf, renderApp, row } from "./App.testSupport";
import { inMemoryRecents } from "./documents/recents";

const now = () => Math.floor(Date.now() / 1000);
const DAY = 86_400;

function panelShows(name: string) {
  const panel = screen.getByRole("region", { name: "Panel de firma" });
  return within(panel).queryByText(name) !== null;
}

async function openPlusMenu(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("button", { name: "Abrir un PDF" }));
  return screen.getByRole("menu");
}

// Grada A: la tira de pestañas y el menú «+», sobre la aplicación entera.
describe("App, con varios documentos abiertos", () => {
  it("keeps several documents open and changes document when the tab changes", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("primero.pdf"), document("segundo.pdf")],
      pdfsOf({ "primero.pdf": 2, "segundo.pdf": 5 }),
    );
    await openPdf(user);
    await openPdf(user);
    await waitFor(() => expect(panelShows("segundo.pdf")).toBe(true));

    await user.click(screen.getByRole("tab", { name: "primero.pdf" }));

    await waitFor(() => expect(panelShows("primero.pdf")).toBe(true));
    expect(screen.getByRole("tab", { name: "primero.pdf", selected: true })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "segundo.pdf", selected: false })).toBeInTheDocument();
  });

  it("names each tab after its file, whole in its tooltip, with a check when it is signed", async () => {
    const user = userEvent.setup();
    const long = `contrato-de-arrendamiento-${"largo-".repeat(8)}.pdf`;
    renderApp(inMemoryRecents(), [document(long, { badge: "Signed" })], pdfsOf({ [long]: 1 }));

    await openPdf(user);

    const tab = await screen.findByRole("tab", { selected: true });
    expect(tab).toHaveTextContent(long);
    expect(tab.closest("[title]")).toHaveAttribute("title", long);
    expect(within(tab).getByRole("img", { name: "Firmado" })).toBeInTheDocument();
  });

  it("closes a tab with its cross and puts the neighbouring document in front", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("primero.pdf"), document("segundo.pdf")],
      pdfsOf({ "primero.pdf": 2, "segundo.pdf": 5 }),
    );
    await openPdf(user);
    await openPdf(user);
    await screen.findByRole("tab", { name: "segundo.pdf", selected: true });

    await user.click(screen.getByRole("button", { name: "Cerrar segundo.pdf" }));

    expect(screen.queryByRole("tab", { name: "segundo.pdf" })).not.toBeInTheDocument();
    await waitFor(() => expect(panelShows("primero.pdf")).toBe(true));
  });

  it("goes back to the drop zone once the last tab is closed", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")], pdfsOf({ "factura.pdf": 2 }));
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

describe("App, el menú «+»", () => {
  it("offers opening a PDF and lists the recents with their date and their check", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents([
        row("hoy.pdf", { lastUsed: now() }),
        row("ayer.pdf", { lastUsed: now() - DAY, badge: "Signed" }),
      ]),
    );
    await screen.findByRole("region", { name: "Recientes" });

    const menu = await openPlusMenu(user);

    expect(within(menu).getByRole("menuitem", { name: "Abrir un PDF…" })).toBeInTheDocument();
    expect(within(menu).getByText("Recientes")).toBeInTheDocument();
    expect(within(menu).getByRole("menuitem", { name: /^hoy\.pdf/ })).toHaveTextContent("hoy");
    const signed = within(menu).getByRole("menuitem", { name: /^ayer\.pdf/ });
    expect(signed).toHaveTextContent("ayer");
    expect(within(signed).getByRole("img", { name: "Firmado" })).toBeInTheDocument();
    expect(within(menu).getByRole("menuitem", { name: "Vaciar la lista" })).toBeInTheDocument();
  });

  it("opens a recent in a new tab", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents([row("a.pdf")]), [], pdfsOf({ "a.pdf": 3 }));
    await screen.findByRole("region", { name: "Recientes" });

    const menu = await openPlusMenu(user);
    await user.click(within(menu).getByRole("menuitem", { name: /^a\.pdf/ }));

    expect(await screen.findByRole("tab", { name: "a.pdf", selected: true })).toBeInTheDocument();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("says Abierto for a recent that already has a tab, and takes the user to it", async () => {
    const user = userEvent.setup();
    renderApp(
      inMemoryRecents(),
      [document("primero.pdf"), document("segundo.pdf")],
      pdfsOf({ "primero.pdf": 2, "segundo.pdf": 5 }),
    );
    await openPdf(user);
    await openPdf(user);
    await screen.findByRole("tab", { name: "segundo.pdf", selected: true });

    const menu = await openPlusMenu(user);
    const open = within(menu).getByRole("menuitem", { name: /^primero\.pdf/ });
    expect(open).toHaveTextContent("Abierto");
    expect(open).toHaveAttribute("title", "Ir a su pestaña");
    await user.click(open);

    expect(screen.getAllByRole("tab")).toHaveLength(2);
    expect(screen.getByRole("tab", { name: "primero.pdf", selected: true })).toBeInTheDocument();
  });

  it("dims a recent that is no longer where it was, and does not open it", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents([row("usb.pdf", { available: false })]));
    await screen.findByRole("region", { name: "Recientes" });

    const menu = await openPlusMenu(user);

    const missing = within(menu).getByRole("menuitem", { name: /^usb\.pdf/ });
    expect(missing).toBeDisabled();
    expect(missing).toHaveTextContent("No se encuentra");
  });

  it("empties the recents from Vaciar la lista, and leaves only Abrir un PDF…", async () => {
    const user = userEvent.setup();
    const recents = inMemoryRecents([row("a.pdf")]);
    renderApp(recents);
    await screen.findByRole("region", { name: "Recientes" });

    await user.click(
      within(await openPlusMenu(user)).getByRole("menuitem", { name: "Vaciar la lista" }),
    );

    await waitFor(() => expect(screen.queryByText("a.pdf")).not.toBeInTheDocument());
    await expect(recents.list()).resolves.toEqual([]);
    const menu = await openPlusMenu(user);
    expect(within(menu).getAllByRole("menuitem")).toHaveLength(1);
  });

  it("closes with Escape", async () => {
    const user = userEvent.setup();
    renderApp();
    await openPlusMenu(user);

    await user.keyboard("{Escape}");

    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });
});

describe("App, sin documentos abiertos", () => {
  it("shows the drop zone and the recents in the middle of the viewer", async () => {
    renderApp(inMemoryRecents([row("a.pdf", { lastUsed: now() })]));

    const viewer = screen.getByRole("region", { name: "Visor del documento" });
    expect(
      within(viewer).getByRole("button", { name: /Arrastra un PDF o pulsa para abrirlo/ }),
    ).toBeInTheDocument();
    const recents = await within(viewer).findByRole("region", { name: "Recientes" });
    expect(within(recents).getByRole("button", { name: /^a\.pdf/ })).toHaveTextContent("hoy");
    expect(within(recents).getByRole("button", { name: "Vaciar la lista" })).toBeInTheDocument();
  });

  it("opens a recent from the empty viewer in a tab", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents([row("a.pdf")]), [], pdfsOf({ "a.pdf": 3 }));
    const recents = await screen.findByRole("region", { name: "Recientes" });

    await user.click(within(recents).getByRole("button", { name: /^a\.pdf/ }));

    expect(await screen.findByRole("tab", { name: "a.pdf", selected: true })).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Recientes" })).not.toBeInTheDocument();
  });

  it("shows only the drop zone when there is nothing recent", async () => {
    renderApp();

    await screen.findByRole("button", { name: /Arrastra un PDF o pulsa para abrirlo/ });
    expect(screen.queryByRole("region", { name: "Recientes" })).not.toBeInTheDocument();
  });
});
