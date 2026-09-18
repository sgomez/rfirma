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
      <Header
        status={null}
        menuAnchor="header"
        onOpenStatus={noop}
        onOpenPreferences={noop}
        onOpenHelp={noop}
        onOpenAbout={noop}
      />,
    );

    expect(screen.queryByText("Sin firmar")).not.toBeInTheDocument();
    expect(screen.queryByText("Firmado")).not.toBeInTheDocument();
  });

  it("shows the cached badge of the open document", () => {
    renderWithCatalog(
      <Header
        status="Unsigned"
        menuAnchor="header"
        onOpenStatus={noop}
        onOpenPreferences={noop}
        onOpenHelp={noop}
        onOpenAbout={noop}
      />,
    );

    expect(screen.getByText("Sin firmar")).toBeInTheDocument();
  });

  // ID-53: el icono es un `<svg>` en línea copiado del artboard, no el `\u2630`
  // de texto que había antes ni un icono de fuente.
  it("draws the menu button with an inline svg icon", () => {
    renderWithCatalog(
      <Header
        status={null}
        menuAnchor="header"
        onOpenStatus={noop}
        onOpenPreferences={noop}
        onOpenHelp={noop}
        onOpenAbout={noop}
      />,
    );

    const button = screen.getByRole("button", { name: "Men\u00fa" });
    expect(button.querySelector("svg")).not.toBeNull();
    expect(button).toHaveTextContent("");
  });

  // El artboard del estado vac\u00edo dibuja el men\u00fa desplegado, pero eso es una
  // posibilidad y no un estado inicial: arranca cerrado.
  it("starts with the menu closed", () => {
    renderWithCatalog(
      <Header
        status={null}
        menuAnchor="header"
        onOpenStatus={noop}
        onOpenPreferences={noop}
        onOpenHelp={noop}
        onOpenAbout={noop}
      />,
    );

    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Men\u00fa" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });

  it("has no menu bar, only the menu button", () => {
    renderWithCatalog(
      <Header
        status={null}
        menuAnchor="header"
        onOpenStatus={noop}
        onOpenPreferences={noop}
        onOpenHelp={noop}
        onOpenAbout={noop}
      />,
    );

    expect(screen.queryByRole("menubar")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Menú" })).toBeInTheDocument();
  });

  it("opens a menu of four entries in two groups with a divider", async () => {
    const user = userEvent.setup();
    renderWithCatalog(
      <Header
        status={null}
        menuAnchor="header"
        onOpenStatus={noop}
        onOpenPreferences={noop}
        onOpenHelp={noop}
        onOpenAbout={noop}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));

    const items = screen.getAllByRole("menuitem");
    expect(items).toHaveLength(4);
    expect(items[0]).toHaveTextContent("Estado de rFirma");
    expect(items[1]).toHaveTextContent("Preferencias…");
    expect(items[2]).toHaveTextContent("Comentarios y ayuda");
    expect(items[3]).toHaveTextContent("Acerca de rFirma");
    expect(screen.getByRole("separator")).toBeInTheDocument();
  });

  it("opens the status view from the menu and closes the menu", async () => {
    const user = userEvent.setup();
    const openStatus = vi.fn();
    renderWithCatalog(
      <Header
        status={null}
        menuAnchor="header"
        onOpenStatus={openStatus}
        onOpenPreferences={noop}
        onOpenHelp={noop}
        onOpenAbout={noop}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Estado de rFirma" }));

    expect(openStatus).toHaveBeenCalledOnce();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("opens the help destination from the menu and closes the menu", async () => {
    const user = userEvent.setup();
    const openHelp = vi.fn();
    renderWithCatalog(
      <Header
        status={null}
        menuAnchor="header"
        onOpenStatus={noop}
        onOpenPreferences={noop}
        onOpenHelp={openHelp}
        onOpenAbout={noop}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: /Comentarios y ayuda/ }));

    expect(openHelp).toHaveBeenCalledOnce();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("reserves the 14px icon column on all entries and shows the external icon on help", async () => {
    const user = userEvent.setup();
    renderWithCatalog(
      <Header
        status={null}
        menuAnchor="header"
        onOpenStatus={noop}
        onOpenPreferences={noop}
        onOpenHelp={noop}
        onOpenAbout={noop}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));

    const items = screen.getAllByRole("menuitem");
    expect(items).toHaveLength(4);
    for (const item of items) {
      expect(item.querySelector(".header__entryIcon")).not.toBeNull();
    }

    const statusItem = screen.getByRole("menuitem", { name: "Estado de rFirma" });
    expect(statusItem.querySelector(".header__entryIcon svg")).toBeNull();

    const helpItem = screen.getByRole("menuitem", { name: /Comentarios y ayuda/ });
    expect(helpItem.querySelector(".header__entryIcon svg")).not.toBeNull();

    const prefItem = screen.getByRole("menuitem", { name: "Preferencias…" });
    expect(prefItem.querySelector(".header__entryIcon svg")).toBeNull();

    const aboutItem = screen.getByRole("menuitem", { name: "Acerca de rFirma" });
    expect(aboutItem.querySelector(".header__entryIcon svg")).toBeNull();
  });

  it("opens the preferences dialog from the menu and closes the menu", async () => {
    const user = userEvent.setup();
    const openPreferences = vi.fn();
    renderWithCatalog(
      <Header
        status={null}
        menuAnchor="header"
        onOpenStatus={noop}
        onOpenPreferences={openPreferences}
        onOpenHelp={noop}
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
      <Header
        status={null}
        menuAnchor="header"
        onOpenStatus={noop}
        onOpenPreferences={noop}
        onOpenHelp={noop}
        onOpenAbout={openAbout}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Acerca de rFirma" }));

    expect(openAbout).toHaveBeenCalledOnce();
  });

  it("closes the open menu with Escape", async () => {
    const user = userEvent.setup();
    renderWithCatalog(
      <Header
        status={null}
        menuAnchor="header"
        onOpenStatus={noop}
        onOpenPreferences={noop}
        onOpenHelp={noop}
        onOpenAbout={noop}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.keyboard("{Escape}");

    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  // El tamaño del botón no es su estado: `cabecera.md` lo fija en 40 px, más
  // estrecho que el mínimo táctil de `.rf-btn`, y ese ancho lo pone
  // `header__button`. Si la clase dependiera de `open`, el botón encogería al
  // abrirse y volvería a crecer al cerrarse.
  it("keeps the menu button sized by header__button whether the menu is open or closed", async () => {
    const user = userEvent.setup();
    renderWithCatalog(
      <Header
        status={null}
        menuAnchor="header"
        onOpenStatus={noop}
        onOpenPreferences={noop}
        onOpenHelp={noop}
        onOpenAbout={noop}
      />,
    );
    const button = screen.getByRole("button", { name: "Menú" });

    expect(button).toHaveClass("header__button");
    expect(button).not.toHaveClass("header__button--open");

    await user.click(button);

    expect(button).toHaveClass("header__button");
    expect(button).toHaveClass("header__button--open");
  });

  it("hides the menu button where the two entries live in the native menu", () => {
    renderWithCatalog(
      <Header
        status={null}
        menuAnchor="native"
        onOpenStatus={noop}
        onOpenPreferences={noop}
        onOpenHelp={noop}
        onOpenAbout={noop}
      />,
    );

    expect(screen.queryByRole("button", { name: "Menú" })).not.toBeInTheDocument();
  });
});
