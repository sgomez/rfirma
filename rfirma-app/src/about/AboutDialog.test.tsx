import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { inMemoryVersionCheck, type NewVersion } from "../updates/newVersion";
import { AboutDialog } from "./AboutDialog";

const noop = () => {};

function renderAbout(props: Partial<Parameters<typeof AboutDialog>[0]> = {}) {
  const newVersion = props.newVersion ?? null;
  return renderWithCatalog(
    <AboutDialog
      version="0.1.0"
      newVersion={newVersion}
      versions={inMemoryVersionCheck(newVersion)}
      offerUpdate
      onOpenSourceCode={noop}
      onClose={noop}
      {...props}
    />,
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

  it("shows both licences without unfolding anything", () => {
    renderAbout();

    expect(screen.getByText("EUPL-1.2")).toBeInTheDocument();
    expect(screen.getByText("Bibliotecas del proyecto Cliente @firma")).toBeInTheDocument();
    expect(screen.getByText("GPL-2.0+ / EUPL-1.1")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Ver las licencias" })).not.toBeInTheDocument();
  });

  it("opens the source code from its link", async () => {
    const user = userEvent.setup();
    const onOpenSourceCode = vi.fn();
    renderAbout({ onOpenSourceCode });

    await user.click(screen.getByRole("link", { name: /github.com\/sgomez\/rfirma/ }));

    expect(onOpenSourceCode).toHaveBeenCalledOnce();
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
      const newVersion: NewVersion = { version: "0.4.1", installable: false };
      renderAbout({ newVersion });

      expect(screen.getByText("Hay una versión nueva: 0.4.1")).toBeInTheDocument();
    });

    it("shows being up to date when there is no new version", () => {
      renderAbout({ newVersion: null });

      expect(screen.getByText("Estás en la última versión")).toBeInTheDocument();
    });

    it("does not tell how to install what is already installed", () => {
      renderAbout({ newVersion: { version: "0.4.1", installable: false } });

      expect(screen.queryByText(/flatpak install/)).not.toBeInTheDocument();
      expect(screen.queryByText(/sudo apt install rfirma/)).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Copiar" })).not.toBeInTheDocument();
    });

    describe("updating from the application", () => {
      const installable: NewVersion = { version: "0.4.1", installable: true };

      it("offers Actualizar ahora only when the answer is installable", () => {
        renderAbout({ newVersion: installable });

        expect(screen.getByRole("button", { name: "Actualizar ahora" })).toBeInTheDocument();
      });

      it("offers nothing when it is not installable", () => {
        renderAbout({ newVersion: { version: "0.4.1", installable: false } });

        expect(screen.queryByRole("button", { name: "Actualizar ahora" })).not.toBeInTheDocument();
      });

      it("keeps showing the status but not the offer when notifications are off", () => {
        renderAbout({ newVersion: installable, offerUpdate: false });

        expect(screen.getByText("Hay una versión nueva: 0.4.1")).toBeInTheDocument();
        expect(screen.queryByRole("button", { name: "Actualizar ahora" })).not.toBeInTheDocument();
      });

      it("confirms with the version before installing", async () => {
        const user = userEvent.setup();
        const versions = inMemoryVersionCheck(installable);
        renderAbout({ newVersion: installable, versions });

        await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));
        expect(screen.getByText("¿Actualizar a la versión 0.4.1?")).toBeInTheDocument();
        expect(versions.installCalls).toBe(0);
        await user.click(screen.getByRole("button", { name: "Instalar y cerrar" }));

        expect(versions.installCalls).toBe(1);
      });

      it("explains a failed installation and keeps About open", async () => {
        const user = userEvent.setup();
        renderAbout({
          newVersion: installable,
          versions: inMemoryVersionCheck(installable, "invalidSignature"),
        });

        await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));
        await user.click(screen.getByRole("button", { name: "Instalar y cerrar" }));

        expect(await screen.findByRole("alert")).toHaveTextContent("no tiene una firma válida");
      });
    });

    it("asks the port again when it opens, and replaces what was known since startup", async () => {
      renderAbout({
        newVersion: null,
        versions: inMemoryVersionCheck({ version: "0.4.1", installable: false }),
      });

      expect(await screen.findByText("Hay una versión nueva: 0.4.1")).toBeInTheDocument();
    });
  });
});
