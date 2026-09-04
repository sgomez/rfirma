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

  /**
   * Cómo actualizar (ID-181): no hay botón de descarga, y la URL solo
   * aparece dentro de una orden copiable.
   */
  describe("how to update", () => {
    it("shows there is a new version, with its number", () => {
      const newVersion: NewVersion = { version: "0.4.1" };
      renderAbout({ newVersion });

      expect(screen.getByText("Hay una versión nueva: 0.4.1")).toBeInTheDocument();
    });

    it("shows being up to date when there is no new version", () => {
      renderAbout({ newVersion: null });

      expect(screen.getByText("Estás en la última versión")).toBeInTheDocument();
    });

    it("shows the enrolment command for the channel selected, Flatpak by default", () => {
      renderAbout();

      expect(screen.getByText(/flatpak install/)).toBeInTheDocument();
      expect(screen.queryByText(/sudo apt install rfirma/)).not.toBeInTheDocument();
    });

    it("switches the command shown when another channel tab is chosen", async () => {
      const user = userEvent.setup();
      renderAbout();

      await user.click(screen.getByRole("tab", { name: "Debian y Ubuntu" }));

      expect(screen.getByText(/sudo apt install rfirma/)).toBeInTheDocument();

      await user.click(screen.getByRole("tab", { name: "Fedora y openSUSE" }));

      expect(screen.getByText(/sudo dnf install rfirma/)).toBeInTheDocument();
    });

    it("never offers a download button, only copyable commands", () => {
      renderAbout();

      expect(screen.queryByRole("button", { name: /descargar/i })).not.toBeInTheDocument();
    });

    it("copies the command of the channel shown to the clipboard", async () => {
      // `userEvent.setup()` sustituye `navigator.clipboard` por su propio
      // doble en cuanto se llama: el espía tiene que engancharse **después**,
      // sobre ese doble, o la sustitución se lo lleva por delante.
      const user = userEvent.setup();
      const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined);
      renderAbout();

      await user.click(screen.getByRole("tab", { name: "Fedora y openSUSE" }));
      await user.click(screen.getByRole("button", { name: "Copiar" }));

      expect(writeText).toHaveBeenCalledWith(expect.stringContaining("sudo dnf install rfirma"));
    });
  });
});
