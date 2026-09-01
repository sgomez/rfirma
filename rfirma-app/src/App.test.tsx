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
import type { PdfDocument, PdfPage, Viewport } from "./viewer/pdf";
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

function renderApp(
  recents = inMemoryRecents(),
  documents: RecentDocument[] = [],
  pdfs: PdfSource = unavailablePdfSource(),
) {
  const preferences = inMemoryPreferences(
    { destination: "Documentos", rememberVisibleSignature: true, rememberActivity: true },
    () => void recents.clear(),
  );
  renderWithCatalog(
    <App
      recents={recents}
      picker={inMemoryDocumentPicker(documents)}
      pdfs={pdfs}
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
    expect(within(panel).getByText(/7/)).toBeInTheDocument();
    const tray = screen.getByRole("region", { name: "Bandeja de documentos" });
    expect(within(tray).getByText("Sin firmar")).toBeInTheDocument();
    // El visor vacío tenía su propia zona de soltar; con el documento pintado
    // solo queda la de la bandeja.
    expect(
      screen.getAllByRole("button", { name: "Arrastra un PDF o pulsa para abrirlo" }),
    ).toHaveLength(1);
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
    expect(within(panel).getByText(/2/)).toBeInTheDocument();
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
