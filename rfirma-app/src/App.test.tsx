import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "./App";
import { inMemoryDocumentPicker } from "./documents/picker";
import { inMemoryRecents, type RecentDocument } from "./documents/recents";
import { inMemoryPreferences } from "./preferences/preferences";
import { renderWithCatalog } from "./testing/render";

function document(path: string, overrides: Partial<RecentDocument> = {}): RecentDocument {
  return {
    path,
    name: path.slice(path.lastIndexOf("/") + 1),
    badge: "Unsigned",
    modified: 1_700_000_000,
    lastUsed: 1_700_000_000,
    available: true,
    ...overrides,
  };
}

function renderApp(recents = inMemoryRecents(), documents: RecentDocument[] = []) {
  const preferences = inMemoryPreferences(
    { destination: "Documentos", rememberVisibleSignature: true, rememberActivity: true },
    () => void recents.clear(),
  );
  renderWithCatalog(
    <App
      recents={recents}
      picker={inMemoryDocumentPicker(documents)}
      preferences={preferences}
      destinations={["Documentos"]}
      menuAnchor="header"
    />,
  );
  return { recents, preferences };
}

// Grada A: la aplicación entera, con los cuatro puertos en memoria.
describe("App", () => {
  it("opens a document from the tray and shows its badge in the header", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("/documentos/factura.pdf")]);

    await user.click(screen.getByRole("button", { name: "Arrastra un PDF o pulsa para abrirlo" }));

    expect(await screen.findByText("factura.pdf")).toBeInTheDocument();
    expect(screen.getByRole("banner")).toHaveTextContent("Sin firmar");
  });

  it("opens Preferences from the menu, over the window and without unmounting it", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents([document("/a.pdf")]));
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
    renderApp(inMemoryRecents([document("/a.pdf")]));
    await screen.findByText("a.pdf");

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    await user.click(await screen.findByRole("switch", { name: /Recordar mi actividad/ }));
    await user.click(screen.getByRole("button", { name: "Apagar y borrar" }));

    await waitFor(() => expect(screen.queryByText("a.pdf")).not.toBeInTheDocument());
    expect(
      screen.getByText("Aquí aparecerán los documentos que vayas firmando"),
    ).toBeInTheDocument();
  });

  it("stops remembering once Remember my activity is off, not just purges what there was", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents([document("/a.pdf")]), [document("/documentos/factura.pdf")]);
    await screen.findByText("a.pdf");

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    await user.click(await screen.findByRole("switch", { name: /Recordar mi actividad/ }));
    await user.click(screen.getByRole("button", { name: "Apagar y borrar" }));
    await waitFor(() => expect(screen.queryByText("a.pdf")).not.toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    await user.click(screen.getByRole("button", { name: "Arrastra un PDF o pulsa para abrirlo" }));

    expect(screen.getByRole("banner")).toHaveTextContent("Sin firmar");
    expect(screen.queryByText("factura.pdf")).not.toBeInTheDocument();
    expect(
      screen.getByText("Aquí aparecerán los documentos que vayas firmando"),
    ).toBeInTheDocument();
  });
});
