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

  describe("keyboard", () => {
    it("opens with the focus on Cerrar and closes on Enter", async () => {
      const user = userEvent.setup();
      const onClose = vi.fn();
      renderAbout({ onClose });

      expect(screen.getByRole("button", { name: "Cerrar" })).toHaveFocus();
      await user.keyboard("{Enter}");

      expect(onClose).toHaveBeenCalledOnce();
    });

    it("closes on Escape", async () => {
      const user = userEvent.setup();
      const onClose = vi.fn();
      renderAbout({ onClose });

      await user.keyboard("{Escape}");

      expect(onClose).toHaveBeenCalledOnce();
    });
  });

  describe("version status", () => {
    describe("updating from the application", () => {
      const installable: NewVersion = { version: "0.4.1", installable: true };

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

      it("opens the confirmation with the focus on Instalar y cerrar and installs on Enter", async () => {
        const user = userEvent.setup();
        const versions = inMemoryVersionCheck(installable);
        const onClose = vi.fn();
        renderAbout({ newVersion: installable, versions, onClose });

        await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));
        expect(screen.getByRole("button", { name: "Instalar y cerrar" })).toHaveFocus();
        await user.keyboard("{Enter}");

        expect(versions.installCalls).toBe(1);
        expect(onClose).not.toHaveBeenCalled();
      });

      it("answers Ahora no on Escape and leaves About open", async () => {
        const user = userEvent.setup();
        const versions = inMemoryVersionCheck(installable);
        const onClose = vi.fn();
        renderAbout({ newVersion: installable, versions, onClose });

        await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));
        await user.keyboard("{Escape}");

        expect(screen.queryByText("¿Actualizar a la versión 0.4.1?")).not.toBeInTheDocument();
        expect(screen.getByRole("dialog", { name: "rFirma" })).toBeInTheDocument();
        expect(versions.installCalls).toBe(0);
        expect(onClose).not.toHaveBeenCalled();
      });

      it("ignores Enter and Escape while installing", async () => {
        const user = userEvent.setup();
        const versions = {
          ...inMemoryVersionCheck(installable),
          install: () => new Promise<never>(() => {}),
        };
        const onClose = vi.fn();
        renderAbout({ newVersion: installable, versions, onClose });

        await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));
        await user.click(screen.getByRole("button", { name: "Instalar y cerrar" }));
        await user.keyboard("{Enter}");
        await user.keyboard("{Escape}");

        expect(screen.getByRole("status")).toBeInTheDocument();
        expect(onClose).not.toHaveBeenCalled();
      });

      it("closes the failure on Enter, with the focus on Cerrar, and leaves About open", async () => {
        const user = userEvent.setup();
        const onClose = vi.fn();
        renderAbout({
          newVersion: installable,
          versions: inMemoryVersionCheck(installable, "invalidSignature"),
          onClose,
        });

        await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));
        await user.click(screen.getByRole("button", { name: "Instalar y cerrar" }));
        await screen.findByRole("alert");
        expect(screen.getAllByRole("button", { name: "Cerrar" }).at(-1)).toHaveFocus();
        await user.keyboard("{Enter}");

        expect(screen.queryByRole("alert")).not.toBeInTheDocument();
        expect(onClose).not.toHaveBeenCalled();
      });

      it("closes the failure on Escape and leaves About open", async () => {
        const user = userEvent.setup();
        const onClose = vi.fn();
        renderAbout({
          newVersion: installable,
          versions: inMemoryVersionCheck(installable, "invalidSignature"),
          onClose,
        });

        await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));
        await user.click(screen.getByRole("button", { name: "Instalar y cerrar" }));
        await screen.findByRole("alert");
        await user.keyboard("{Escape}");

        expect(screen.queryByRole("alert")).not.toBeInTheDocument();
        expect(onClose).not.toHaveBeenCalled();
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
