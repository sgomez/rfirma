import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { Header } from "./Header";

const noop = () => {};

// Grada A: React, jsdom y el catálogo. Nada de token ni de puente.
describe("Header", () => {
  it("shows no status badge while no document is open", () => {
    renderWithCatalog(
      <Header status={null} menuAnchor="header" onOpenPreferences={noop} onOpenAbout={noop} />,
    );

    expect(screen.queryByText("Sin firmar")).not.toBeInTheDocument();
    expect(screen.queryByText("Firmado")).not.toBeInTheDocument();
  });

  it("shows the cached badge of the open document", () => {
    renderWithCatalog(
      <Header status="Unsigned" menuAnchor="header" onOpenPreferences={noop} onOpenAbout={noop} />,
    );

    expect(screen.getByText("Sin firmar")).toBeInTheDocument();
  });

  it("has no menu bar, only the menu button", () => {
    renderWithCatalog(
      <Header status={null} menuAnchor="header" onOpenPreferences={noop} onOpenAbout={noop} />,
    );

    expect(screen.queryByRole("menubar")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Menú" })).toBeInTheDocument();
  });

  it("opens a menu of exactly two entries", async () => {
    const user = userEvent.setup();
    renderWithCatalog(
      <Header status={null} menuAnchor="header" onOpenPreferences={noop} onOpenAbout={noop} />,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));

    expect(screen.getAllByRole("menuitem")).toHaveLength(2);
    expect(screen.getByRole("menuitem", { name: "Preferencias…" })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Acerca de rFirma" })).toBeInTheDocument();
  });

  it("opens the preferences dialog from the menu and closes the menu", async () => {
    const user = userEvent.setup();
    const openPreferences = vi.fn();
    renderWithCatalog(
      <Header
        status={null}
        menuAnchor="header"
        onOpenPreferences={openPreferences}
        onOpenAbout={noop}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Preferencias…" }));

    expect(openPreferences).toHaveBeenCalledOnce();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("opens the about dialog from the menu", async () => {
    const user = userEvent.setup();
    const openAbout = vi.fn();
    renderWithCatalog(
      <Header status={null} menuAnchor="header" onOpenPreferences={noop} onOpenAbout={openAbout} />,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Acerca de rFirma" }));

    expect(openAbout).toHaveBeenCalledOnce();
  });

  it("closes the open menu with Escape", async () => {
    const user = userEvent.setup();
    renderWithCatalog(
      <Header status={null} menuAnchor="header" onOpenPreferences={noop} onOpenAbout={noop} />,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.keyboard("{Escape}");

    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("hides the menu button where the two entries live in the native menu", () => {
    renderWithCatalog(
      <Header status={null} menuAnchor="native" onOpenPreferences={noop} onOpenAbout={noop} />,
    );

    expect(screen.queryByRole("button", { name: "Menú" })).not.toBeInTheDocument();
  });
});
