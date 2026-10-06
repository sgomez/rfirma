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

  it("closes on Intro and on Escape", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderAbout({ onClose });

    expect(screen.getByRole("button", { name: "Cerrar" })).toHaveFocus();
    await user.keyboard("{Enter}");
    await user.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalledTimes(2);
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

      it("answers Intro with Instalar y cerrar and Escape with Ahora no, only on the update", async () => {
        const user = userEvent.setup();
        const onClose = vi.fn();
        const versions = inMemoryVersionCheck(installable);
        renderAbout({ newVersion: installable, versions, onClose });

        await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));
        expect(screen.getByRole("button", { name: "Instalar y cerrar" })).toHaveFocus();
        await user.keyboard("{Escape}");
        expect(screen.queryByText("¿Actualizar a la versión 0.4.1?")).not.toBeInTheDocument();
        expect(onClose).not.toHaveBeenCalled();

        await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));
        await user.keyboard("{Enter}");

        expect(versions.installCalls).toBe(1);
        expect(onClose).not.toHaveBeenCalled();
      });

      it("ignores Intro and Escape while installing", async () => {
        const user = userEvent.setup();
        const onClose = vi.fn();
        const versions = {
          ...inMemoryVersionCheck(installable),
          install: vi.fn(() => new Promise<never>(() => {})),
        };
        renderAbout({ newVersion: installable, versions, onClose });

        await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));
        await user.keyboard("{Enter}");
        expect(versions.install).toHaveBeenCalledOnce();
        await user.keyboard("{Enter}");
        await user.keyboard("{Escape}");

        expect(versions.install).toHaveBeenCalledOnce();
        expect(screen.getByRole("status")).toBeInTheDocument();
        expect(onClose).not.toHaveBeenCalled();
      });

      it("closes the update with Intro and with Escape after a failure", async () => {
        const user = userEvent.setup();
        renderAbout({
          newVersion: installable,
          versions: inMemoryVersionCheck(installable, "invalidSignature"),
        });

        for (const key of ["{Enter}", "{Escape}"]) {
          await user.click(screen.getByRole("button", { name: "Actualizar ahora" }));
          await user.click(screen.getByRole("button", { name: "Instalar y cerrar" }));
          expect(await screen.findByRole("alert")).toBeInTheDocument();
          await user.keyboard(key);
          expect(screen.queryByRole("alert")).not.toBeInTheDocument();
        }
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
