import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { useActionKeys } from "./actionKeys";
import { Button } from "./Button";
import { Dialog } from "./Dialog";
import { Select } from "./Select";

function Screen({
  onPrimary,
  onSecondary,
  children,
}: {
  onPrimary: () => void;
  onSecondary: () => void;
  children?: React.ReactNode;
}) {
  const primary = useRef<HTMLButtonElement>(null);
  useActionKeys({ primary, onSecondary });
  return (
    <>
      <Button ref={primary} onClick={onPrimary}>
        Continuar
      </Button>
      {children}
    </>
  );
}

function Confirm({
  label = "¿Firmar de todos modos?",
  onSign = () => {},
  onCancel = () => {},
  disabled = false,
  children,
}: {
  label?: string;
  onSign?: () => void;
  onCancel?: () => void;
  disabled?: boolean;
  children?: React.ReactNode;
}) {
  const primary = useRef<HTMLButtonElement>(null);
  return (
    <Dialog label={label} onClose={onCancel} primary={primary}>
      {children}
      <Button onClick={onCancel}>Cancelar</Button>
      <Button ref={primary} variant="primary" disabled={disabled} onClick={onSign}>
        Firmar
      </Button>
    </Dialog>
  );
}

function InlineConfirm({ onCancel }: { onCancel: () => void }) {
  useActionKeys({ onSecondary: onCancel });
  return <p>¿Vaciar el almacén?</p>;
}

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

  it("presses its primary on Enter, with the focus on it on opening", async () => {
    const onSign = vi.fn();
    render(<Confirm onSign={onSign} />);

    expect(screen.getByRole("button", { name: "Firmar" })).toHaveFocus();
    screen.getByRole("dialog").focus();
    await userEvent.keyboard("{Enter}");

    expect(onSign).toHaveBeenCalledOnce();
  });

  it("ignores Enter when it has no primary", async () => {
    const onSign = vi.fn();
    render(
      <Dialog label="Firmas" onClose={() => {}}>
        <Button onClick={onSign}>Firmar</Button>
      </Dialog>,
    );

    await userEvent.keyboard("{Enter}");

    expect(onSign).not.toHaveBeenCalled();
  });

  it("presses its secondary on Escape", async () => {
    const onCancel = vi.fn();
    const onSign = vi.fn();
    render(<Confirm onSign={onSign} onCancel={onCancel} />);

    await userEvent.keyboard("{Escape}");

    expect(onCancel).toHaveBeenCalledOnce();
    expect(onSign).not.toHaveBeenCalled();
  });

  it("ignores Enter while the primary is disabled and focuses it once enabled", async () => {
    const onSign = vi.fn();
    const { rerender } = render(<Confirm onSign={onSign} disabled />);

    await userEvent.keyboard("{Enter}");
    expect(onSign).not.toHaveBeenCalled();

    rerender(<Confirm onSign={onSign} />);
    expect(screen.getByRole("button", { name: "Firmar" })).toHaveFocus();
  });

  it("keeps the focus where the person moved it when the primary becomes enabled", async () => {
    const { rerender } = render(<Confirm disabled />);

    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Cancelar" })).toHaveFocus();
    rerender(<Confirm />);

    expect(screen.getByRole("button", { name: "Cancelar" })).toHaveFocus();
  });

  it("leaves Enter to the focused button", async () => {
    const onSign = vi.fn();
    const onCancel = vi.fn();
    render(<Confirm onSign={onSign} onCancel={onCancel} />);

    screen.getByRole("button", { name: "Cancelar" }).focus();
    await userEvent.keyboard("{Enter}");

    expect(onCancel).toHaveBeenCalledOnce();
    expect(onSign).not.toHaveBeenCalled();
  });

  it("leaves Enter to the focused text field", async () => {
    const onSign = vi.fn();
    render(
      <Confirm onSign={onSign}>
        <input aria-label="Páginas" />
      </Confirm>,
    );

    await userEvent.click(screen.getByRole("textbox", { name: "Páginas" }));
    await userEvent.keyboard("{Enter}");

    expect(onSign).not.toHaveBeenCalled();
  });

  it("leaves Enter to the focused dropdown", async () => {
    const onSign = vi.fn();
    render(
      <Confirm onSign={onSign}>
        <Select
          label="Certificado"
          value="a"
          options={[
            { value: "a", label: "Uno" },
            { value: "b", label: "Dos" },
          ]}
          onChange={() => {}}
        />
      </Confirm>,
    );

    screen.getByRole("combobox", { name: "Certificado" }).focus();
    await userEvent.keyboard("{Enter}");
    expect(screen.getByRole("listbox")).toBeInTheDocument();
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

    screen.getByRole("dialog", { name: "Dentro" }).focus();
    await userEvent.keyboard("{Enter}");

    expect(onInner).toHaveBeenCalledOnce();
    expect(onOuter).not.toHaveBeenCalled();
  });

  it("keeps the screen behind it from answering Enter and Escape", async () => {
    const onPrimary = vi.fn();
    const onSecondary = vi.fn();
    const { rerender } = render(
      <Screen onPrimary={onPrimary} onSecondary={onSecondary}>
        <Dialog label="Firmando">Firmando</Dialog>
      </Screen>,
    );

    await userEvent.keyboard("{Enter}{Escape}");
    expect(onPrimary).not.toHaveBeenCalled();
    expect(onSecondary).not.toHaveBeenCalled();

    rerender(<Screen onPrimary={onPrimary} onSecondary={onSecondary} />);
    await userEvent.keyboard("{Enter}{Escape}");
    expect(onPrimary).toHaveBeenCalledOnce();
    expect(onSecondary).toHaveBeenCalledOnce();
  });

  it("does not compile without a label", () => {
    // @ts-expect-error la etiqueta accesible es obligatoria
    render(<Dialog>x</Dialog>);
  });

  it("lets an inline confirmation opened later answer Escape before its screen", async () => {
    const onSecondary = vi.fn();
    const onCancel = vi.fn();
    const { rerender } = render(<Screen onPrimary={() => {}} onSecondary={onSecondary} />);

    rerender(
      <Screen onPrimary={() => {}} onSecondary={onSecondary}>
        <InlineConfirm onCancel={onCancel} />
      </Screen>,
    );
    await userEvent.keyboard("{Escape}");

    expect(onCancel).toHaveBeenCalledOnce();
    expect(onSecondary).not.toHaveBeenCalled();
  });

  it("lets an inline confirmation mounted with its screen answer Escape first", async () => {
    const onSecondary = vi.fn();
    const onCancel = vi.fn();
    render(
      <Screen onPrimary={() => {}} onSecondary={onSecondary}>
        <InlineConfirm onCancel={onCancel} />
      </Screen>,
    );

    await userEvent.keyboard("{Escape}");

    expect(onCancel).toHaveBeenCalledOnce();
    expect(onSecondary).not.toHaveBeenCalled();
  });
});
