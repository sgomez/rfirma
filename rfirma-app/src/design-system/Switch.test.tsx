import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Switch } from "./Switch";

/** **Grada A**: un interruptor y su teclado. */
describe("Switch", () => {
  it("is a switch named by its label and reports its state", () => {
    render(<Switch label="Recordar" checked onChange={vi.fn()} />);

    expect(screen.getByRole("switch", { name: "Recordar" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
  });

  it("toggles with the pointer", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Switch label="Recordar" checked={false} onChange={onChange} />);

    await user.click(screen.getByRole("switch", { name: "Recordar" }));

    expect(onChange).toHaveBeenCalledWith(true);
  });

  it.each([["{Enter}"], [" "]])("toggles with the key %j", async (key) => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Switch label="Recordar" checked onChange={onChange} />);

    await user.tab();
    await user.keyboard(key);

    expect(onChange).toHaveBeenCalledWith(false);
  });

  it("is named by another element when it has no text of its own", () => {
    render(
      <>
        <p id="owner">Cuenta atrás</p>
        <Switch labelledBy="owner" checked={false} onChange={vi.fn()} />
      </>,
    );

    expect(screen.getByRole("switch", { name: "Cuenta atrás" })).toBeInTheDocument();
  });

  it("can be on and disabled at once, and then does not toggle", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Switch label="Rúbrica" checked disabled title="Bloqueada" onChange={onChange} />);

    const control = screen.getByRole("switch", { name: "Rúbrica" });
    await user.click(control);

    expect(control).toBeDisabled();
    expect(control).toHaveAttribute("aria-checked", "true");
    expect(control).toHaveAttribute("title", "Bloqueada");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("describes the switch with its hint without adding it to the name", () => {
    render(<Switch label="Recordar" hint="Se guarda en el equipo" checked onChange={vi.fn()} />);

    const control = screen.getByRole("switch", { name: "Recordar" });
    expect(control).toHaveAccessibleDescription("Se guarda en el equipo");
  });

  it("does not compile without an accessible name", () => {
    // @ts-expect-error ni `label` ni `labelledBy`
    const unnamed = <Switch checked onChange={vi.fn()} />;
    expect(unnamed).toBeDefined();
  });
});
