import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { type ReactNode, useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { useDefaultButton } from "./actionKeys";
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

function Confirm({
  onSign,
  onCancel,
  ready = true,
  label = "¿Firmar?",
  children,
}: {
  onSign: () => void;
  onCancel?: () => void;
  ready?: boolean;
  label?: string;
  children?: ReactNode;
}) {
  const sign = useDefaultButton(ready);
  return (
    <Dialog label={label} onClose={onCancel} primary={sign}>
      {children}
      <button type="button" onClick={onCancel}>
        Cancelar
      </button>
      <button type="button" ref={sign} disabled={!ready} onClick={onSign}>
        Firmar
      </button>
    </Dialog>
  );
}

function ReadyLater({ onSign }: { onSign: () => void }) {
  const [ready, setReady] = useState(false);
  return (
    <>
      <button type="button" onClick={() => setReady(true)}>
        Activar
      </button>
      <Confirm onSign={onSign} ready={ready} />
    </>
  );
}

describe("Dialog actions on the keyboard", () => {
  it("presses its primary on Enter and focuses it on opening", async () => {
    const onSign = vi.fn();
    render(<Confirm onSign={onSign} />);

    expect(screen.getByRole("button", { name: "Firmar" })).toHaveFocus();
    await userEvent.keyboard("{Enter}");

    expect(onSign).toHaveBeenCalledOnce();
  });

  it("does nothing on Enter without a primary", async () => {
    const onPress = vi.fn();
    render(
      <Dialog label="Acerca de">
        <button type="button" onClick={onPress}>
          Cerrar
        </button>
      </Dialog>,
    );

    await userEvent.keyboard("{Enter}");

    expect(screen.getByRole("dialog")).toHaveFocus();
    expect(onPress).not.toHaveBeenCalled();
  });

  it("presses its secondary on Escape", async () => {
    const onSign = vi.fn();
    const onCancel = vi.fn();
    render(<Confirm onSign={onSign} onCancel={onCancel} />);

    await userEvent.keyboard("{Escape}");

    expect(onCancel).toHaveBeenCalledOnce();
    expect(onSign).not.toHaveBeenCalled();
  });

  it("ignores Enter while the primary is disabled and focuses it once enabled", async () => {
    const onSign = vi.fn();
    render(<ReadyLater onSign={onSign} />);

    await userEvent.keyboard("{Enter}");
    expect(onSign).not.toHaveBeenCalled();

    screen.getByRole("button", { name: "Activar", hidden: true }).click();

    expect(await screen.findByRole("button", { name: "Firmar" })).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(onSign).toHaveBeenCalledOnce();
  });

  it("leaves Enter to the button with the focus", async () => {
    const onSign = vi.fn();
    const onCancel = vi.fn();
    render(<Confirm onSign={onSign} onCancel={onCancel} />);

    screen.getByRole("button", { name: "Cancelar" }).focus();
    await userEvent.keyboard("{Enter}");

    expect(onCancel).toHaveBeenCalledOnce();
    expect(onSign).not.toHaveBeenCalled();
  });

  it("leaves Enter to a text field or a dropdown with the focus", async () => {
    const onSign = vi.fn();
    render(
      <Confirm onSign={onSign}>
        <input aria-label="Motivo" />
        <select aria-label="Formato">
          <option>PAdES</option>
        </select>
      </Confirm>,
    );

    screen.getByRole("textbox", { name: "Motivo" }).focus();
    await userEvent.keyboard("{Enter}");
    screen.getByRole("combobox", { name: "Formato" }).focus();
    await userEvent.keyboard("{Enter}");

    expect(onSign).not.toHaveBeenCalled();
  });

  it("lets only the most recent one answer Enter", async () => {
    const onOuter = vi.fn();
    const onInner = vi.fn();
    render(
      <>
        <Confirm label="Fuera" onSign={onOuter} />
        <Confirm label="Dentro" onSign={onInner} />
      </>,
    );

    await userEvent.keyboard("{Enter}");

    expect(onInner).toHaveBeenCalledOnce();
    expect(onOuter).not.toHaveBeenCalled();
  });
});

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

  it("answers Escape before a listener of the view behind it", async () => {
    const onView = vi.fn();
    const onClose = vi.fn();
    const listener = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) onView();
    };
    window.addEventListener("keydown", listener);
    render(<Dialog label="Acerca de" onClose={onClose} />);

    await userEvent.keyboard("{Escape}");
    window.removeEventListener("keydown", listener);

    expect(onClose).toHaveBeenCalledOnce();
    expect(onView).not.toHaveBeenCalled();
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
