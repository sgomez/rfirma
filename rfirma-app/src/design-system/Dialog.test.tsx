import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Dialog } from "./Dialog";

function Pair({ onOuter, onInner }: { onOuter: () => void; onInner: () => void }) {
  return (
    <>
      <Dialog label="Fuera" onClose={onOuter}>
        <button type="button">Uno</button>
      </Dialog>
      <Dialog label="Dentro" onClose={onInner}>
        <button type="button">Dos</button>
      </Dialog>
    </>
  );
}

describe("Dialog", () => {
  it("is a modal named by its label, with focus on itself on opening", () => {
    render(
      <Dialog label="Acerca de">
        <button type="button">Cerrar</button>
      </Dialog>,
    );

    const dialog = screen.getByRole("dialog", { name: "Acerca de" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(dialog).toHaveFocus();
  });

  it("closes on Escape", async () => {
    const onClose = vi.fn();
    render(
      <Dialog label="Acerca de" onClose={onClose}>
        <button type="button">Cerrar</button>
      </Dialog>,
    );

    await userEvent.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("ignores Escape when it has no way out", async () => {
    render(
      <Dialog label="Firmando">
        <p>Firmando</p>
      </Dialog>,
    );

    await userEvent.keyboard("{Escape}");

    expect(screen.getByRole("dialog", { name: "Firmando" })).toBeInTheDocument();
  });

  it("keeps Tab and Shift+Tab inside", async () => {
    const user = userEvent.setup();
    render(
      <>
        <button type="button">Detrás</button>
        <Dialog label="Acerca de" onClose={() => {}}>
          <button type="button">Primero</button>
          <button type="button">Último</button>
        </Dialog>
      </>,
    );

    await user.tab();
    expect(screen.getByRole("button", { name: "Primero" })).toHaveFocus();
    await user.tab();
    await user.tab();
    expect(screen.getByRole("button", { name: "Primero" })).toHaveFocus();
    await user.tab({ shift: true });
    expect(screen.getByRole("button", { name: "Último" })).toHaveFocus();
  });

  it("lets only the most recent one answer Escape", async () => {
    const onOuter = vi.fn();
    const onInner = vi.fn();
    render(<Pair onOuter={onOuter} onInner={onInner} />);

    await userEvent.keyboard("{Escape}");

    expect(onInner).toHaveBeenCalledOnce();
    expect(onOuter).not.toHaveBeenCalled();
  });

  it("gives the focus back to what had it before opening", () => {
    const { rerender } = render(<button type="button">Abrir</button>);
    screen.getByRole("button", { name: "Abrir" }).focus();

    rerender(
      <>
        <button type="button">Abrir</button>
        <Dialog label="Acerca de">x</Dialog>
      </>,
    );
    expect(screen.getByRole("dialog")).toHaveFocus();

    rerender(<button type="button">Abrir</button>);
    expect(screen.getByRole("button", { name: "Abrir" })).toHaveFocus();
  });

  it("does not compile without a label", () => {
    // @ts-expect-error la etiqueta accesible es obligatoria
    render(<Dialog>x</Dialog>);
  });
});
