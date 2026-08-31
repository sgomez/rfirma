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
        panel={null}
      />,
    );

    expect(screen.getByRole("banner")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Bandeja de documentos" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Visor del documento" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Panel de firma" })).toBeInTheDocument();
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
        panel={null}
      />,
    );

    expect(screen.getByText("Firmado")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Bandeja de documentos" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Panel de firma" })).toBeInTheDocument();
  });
});
