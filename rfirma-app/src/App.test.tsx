import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "./App";
import { inMemoryDocumentPicker } from "./documents/picker";
import { inMemoryRecents, type RecentDocument } from "./documents/recents";
import { inMemoryPreferences } from "./preferences/preferences";
import { emptyCertificateStore } from "./signing/certificate";
import { unavailableSigningBackend } from "./signing/flow";
import { emptyRubricPicker } from "./signing/rubric";
import { emptyLayer2Composer } from "./signing/visibleSignature";
import { renderWithCatalog } from "./testing/render";
import { emptyPdfSource } from "./viewer/source";

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
      pdfs={emptyPdfSource()}
      preferences={preferences}
      destinations={["Documentos"]}
      certificates={emptyCertificateStore()}
      rubrics={emptyRubricPicker()}
      composer={emptyLayer2Composer()}
      signer={unavailableSigningBackend()}
      menuAnchor="header"
    />,
  );
  return { recents, preferences };
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
  it("opens a document from the tray and shows its badge in the header", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")]);

    await user.click(trayDropZone());

    expect(await screen.findByText("factura.pdf")).toBeInTheDocument();
    expect(screen.getByRole("banner")).toHaveTextContent("Sin firmar");
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
    await user.click(screen.getByRole("button", { name: "Apagar y borrar" }));

    await waitFor(() => expect(screen.queryByText("a.pdf")).not.toBeInTheDocument());
    expect(
      screen.getByText("Aquí aparecerán los documentos que vayas firmando"),
    ).toBeInTheDocument();
  });

  it("stops remembering once Remember my activity is off, not just purges what there was", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents([document("a.pdf")]), [document("factura.pdf")]);
    await screen.findByText("a.pdf");

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));
    await user.click(await screen.findByRole("switch", { name: /Recordar mi actividad/ }));
    await user.click(screen.getByRole("button", { name: "Apagar y borrar" }));
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
