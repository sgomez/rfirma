import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import type { NewVersion } from "../updates/newVersion";
import { AboutDialog } from "./AboutDialog";

const noop = () => {};

function renderAbout(props: Partial<Parameters<typeof AboutDialog>[0]> = {}) {
  return renderWithCatalog(
    <AboutDialog version="0.1.0" newVersion={null} onClose={noop} {...props} />,
  );
}

// Grada A. Lo que se comprueba es el **contenido** obligatorio, no la estética.
describe("AboutDialog", () => {
  it("declares that rFirma is not the official client", () => {
    renderAbout();

    const notice = screen.getByText(/Proyecto independiente/);
    expect(notice).toHaveTextContent(/no está relacionada con AutoFirma/);
    expect(notice).toHaveTextContent(/ni cuenta con su respaldo/);
    expect(notice).toHaveTextContent(
      "Si detectas algún problema con rFirma, comunícalo en nuestro repositorio y no al equipo de AutoFirma.",
    );
  });

  /**
   * ID-211: la frase «el documento y la clave privada no salen de tu
   * ordenador» tranquiliza sobre lo evidente y se retiró.
   */
  it("does not narrate that the document and the private key stay on the computer", () => {
    renderAbout();

    expect(screen.queryByText(/no salen de tu ordenador/)).not.toBeInTheDocument();
  });

  it("shows the version", () => {
    renderAbout();

    expect(screen.getByText("Versión 0.1.0")).toBeInTheDocument();
  });

  it("shows both licences", async () => {
    const user = userEvent.setup();
    renderAbout();

    await user.click(screen.getByRole("button", { name: "Ver las licencias" }));

    expect(screen.getByText("rFirma: EUPL-1.2.")).toBeInTheDocument();
    expect(
      screen.getByText("Bibliotecas de Cliente @firma: GPL-2.0+ / EUPL-1.1."),
    ).toBeInTheDocument();
  });

  it("closes on Cerrar", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderAbout({ onClose });

    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(onClose).toHaveBeenCalledOnce();
  });

  describe("version status", () => {
    it("shows there is a new version, with its number", () => {
      const newVersion: NewVersion = { version: "0.4.1" };
      renderAbout({ newVersion });

      expect(screen.getByText("Hay una versión nueva: 0.4.1")).toBeInTheDocument();
    });

    it("shows being up to date when there is no new version", () => {
      renderAbout({ newVersion: null });

      expect(screen.getByText("Estás en la última versión")).toBeInTheDocument();
    });

    it("does not tell how to install what is already installed", () => {
      renderAbout({ newVersion: { version: "0.4.1" } });

      expect(screen.queryByText(/flatpak install/)).not.toBeInTheDocument();
      expect(screen.queryByText(/sudo apt install rfirma/)).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Copiar" })).not.toBeInTheDocument();
    });
  });
});
