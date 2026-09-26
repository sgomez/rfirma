import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { MainWindow } from "./MainWindow";

const noop = () => {};

// Grada A. Lo que se comprueba aquí es la **estructura** del ID-25, no el
// aspecto: las tres regiones están siempre, y no hay navegación.
describe("MainWindow", () => {
  it("lays out the viewer and the panel under the header", () => {
    renderWithCatalog(
      <MainWindow
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tabs={null}
        viewer={null}
        panel={<p>panel</p>}
      />,
    );

    expect(screen.getByRole("banner")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Visor del documento" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Panel de firma" })).toBeInTheDocument();
  });

  it("passes hasAttention through to the header menu", async () => {
    const user = userEvent.setup();
    renderWithCatalog(
      <MainWindow
        menuAnchor="header"
        hasAttention
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tabs={null}
        viewer={null}
        panel={null}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));

    expect(screen.getByRole("menuitem", { name: /Estado de rFirma/ })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Requiere atención" })).toBeInTheDocument();
  });

  // ID-51: sin documento el panel **no se monta**. La ventana pasa a una
  // columna, que es lo que dice el estado 1 de la tabla de la ficha
  // (`oculto`) y lo que enseña el artboard del estado vacío.
  it("does not mount the signing panel while there is no document", () => {
    renderWithCatalog(
      <MainWindow
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tabs={null}
        viewer={null}
        panel={null}
      />,
    );

    expect(screen.queryByRole("region", { name: "Panel de firma" })).not.toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Visor del documento" })).toBeInTheDocument();
  });

  it("mounts the tab strip under the header", () => {
    renderWithCatalog(
      <MainWindow
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tabs={<nav aria-label="Documentos abiertos">contrato.pdf</nav>}
        viewer={null}
        panel={null}
      />,
    );

    const tabs = screen.getByRole("navigation", { name: "Documentos abiertos" });
    expect(tabs).toContainElement(screen.getByText("contrato.pdf"));
  });

  it("puts the viewer content inside the viewer region", () => {
    renderWithCatalog(
      <MainWindow
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tabs={null}
        viewer={<p>página 3 de 27</p>}
        panel={null}
      />,
    );

    const viewer = screen.getByRole("region", { name: "Visor del documento" });
    expect(viewer).toContainElement(screen.getByText("página 3 de 27"));
  });

  it("has no navigation between screens", () => {
    renderWithCatalog(
      <MainWindow
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tabs={null}
        viewer={null}
        panel={null}
      />,
    );

    expect(screen.queryByRole("navigation")).not.toBeInTheDocument();
    expect(screen.queryAllByRole("link")).toHaveLength(0);
  });

  it("keeps the three regions when a document is open", () => {
    renderWithCatalog(
      <MainWindow
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tabs={null}
        viewer={null}
        panel={<p>panel</p>}
      />,
    );

    expect(screen.getByRole("region", { name: "Visor del documento" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Panel de firma" })).toBeInTheDocument();
  });

  // ID-207: el hueco de la franja está entre la cabecera y las regiones, y
  // sólo hay franja cuando hay algo que notificar.
  it("mounts nothing between the header and the regions while there is nothing to notify", () => {
    const { container } = renderWithCatalog(
      <MainWindow
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tabs={null}
        viewer={null}
        panel={null}
      />,
    );

    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(container.querySelector(".main-window")?.children).toHaveLength(2);
  });

  it("puts the notification strip under the header and over the regions", () => {
    const { container } = renderWithCatalog(
      <MainWindow
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        notification={<p role="status">hay algo que contar</p>}
        tabs={null}
        viewer={null}
        panel={null}
      />,
    );

    const window = container.querySelector(".main-window");
    const [header, strip, body] = [...(window?.children ?? [])];
    expect(window?.children).toHaveLength(3);
    expect(header?.tagName).toBe("HEADER");
    expect(strip).toHaveTextContent("hay algo que contar");
    expect(body).toHaveClass("main-window__body");
  });

  it("mounts a body view under the header instead of the tabs and the regions", () => {
    renderWithCatalog(
      <MainWindow
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        view={<div data-testid="body-view">vista de estado</div>}
        tabs={<nav aria-label="Documentos abiertos" />}
        viewer={<p>viewer</p>}
        panel={<p>panel</p>}
      />,
    );

    expect(screen.getByRole("banner")).toBeInTheDocument();
    expect(screen.getByTestId("body-view")).toBeInTheDocument();
    expect(
      screen.queryByRole("navigation", { name: "Documentos abiertos" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Visor del documento" })).not.toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Panel de firma" })).not.toBeInTheDocument();
  });
});
