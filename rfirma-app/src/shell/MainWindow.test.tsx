import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { MainWindow } from "./MainWindow";

const noop = () => {};

// Grada A. Lo que se comprueba aquí es la **estructura** del ID-25, no el
// aspecto: las tres regiones están siempre, y no hay navegación.
describe("MainWindow", () => {
  it("lays out the three regions under the header", () => {
    renderWithCatalog(
      <MainWindow
        status={null}
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tray={null}
        viewer={null}
        panel={<p>panel</p>}
      />,
    );

    expect(screen.getByRole("banner")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Bandeja de documentos" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Visor del documento" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Panel de firma" })).toBeInTheDocument();
  });

  // ID-51: sin documento el panel **no se monta**. La ventana pasa a dos
  // columnas, que es lo que dice el estado 1 de la tabla de la ficha
  // (`oculto`) y lo que enseña el artboard del estado vacío.
  it("does not mount the signing panel while there is no document", () => {
    renderWithCatalog(
      <MainWindow
        status={null}
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tray={null}
        viewer={null}
        panel={null}
      />,
    );

    expect(screen.queryByRole("region", { name: "Panel de firma" })).not.toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Bandeja de documentos" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Visor del documento" })).toBeInTheDocument();
  });

  it("puts the tray content inside the tray region", () => {
    renderWithCatalog(
      <MainWindow
        status={null}
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tray={<p>contrato.pdf</p>}
        viewer={null}
        panel={null}
      />,
    );

    const tray = screen.getByRole("region", { name: "Bandeja de documentos" });
    expect(tray).toContainElement(screen.getByText("contrato.pdf"));
  });

  it("puts the viewer content inside the viewer region", () => {
    renderWithCatalog(
      <MainWindow
        status={null}
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tray={null}
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
        status="Unsigned"
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tray={null}
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
        status="Signed"
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tray={null}
        viewer={null}
        panel={<p>panel</p>}
      />,
    );

    expect(screen.getByText("Firmado")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Bandeja de documentos" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Panel de firma" })).toBeInTheDocument();
  });

  // ID-207: el hueco de la franja está entre la cabecera y las regiones, y
  // sólo hay franja cuando hay algo que notificar.
  it("mounts nothing between the header and the regions while there is nothing to notify", () => {
    const { container } = renderWithCatalog(
      <MainWindow
        status={null}
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        tray={null}
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
        status={null}
        menuAnchor="header"
        onOpenPreferences={noop}
        onOpenAbout={noop}
        notification={<p role="status">hay algo que contar</p>}
        tray={null}
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
});
