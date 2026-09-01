import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Select } from "./Select";

const options = [
  { value: "system", label: "El del sistema" },
  { value: "light", label: "Claro" },
  { value: "dark", label: "Oscuro" },
];

function renderSelect(onChange = vi.fn(), value = "system") {
  render(<Select label="Tema" value={value} options={options} onChange={onChange} />);
  return onChange;
}

/**
 * **Grada A**: un componente y su teclado.
 *
 * Lo que se comprueba aquí es justo lo que se perdió al dejar el `<select>`
 * nativo: sin estas pruebas, un desplegable propio es un `<div>` que se pinta
 * bien y no se puede usar sin ratón.
 */
describe("Select", () => {
  it("shows what is chosen without opening anything", () => {
    renderSelect();

    const control = screen.getByRole("combobox", { name: "Tema" });
    expect(control).toHaveTextContent("El del sistema");
    expect(control).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("chooses with the pointer", async () => {
    const user = userEvent.setup();
    const onChange = renderSelect();

    await user.click(screen.getByRole("combobox", { name: "Tema" }));
    await user.click(screen.getByRole("option", { name: "Oscuro" }));

    expect(onChange).toHaveBeenCalledWith("dark");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("opens with the down arrow and chooses with Enter", async () => {
    const user = userEvent.setup();
    const onChange = renderSelect();

    screen.getByRole("combobox", { name: "Tema" }).focus();
    await user.keyboard("{ArrowDown}{ArrowDown}{Enter}");

    expect(onChange).toHaveBeenCalledWith("light");
  });

  /** Abrir no es empezar de cero: el cursor arranca en lo ya elegido. */
  it("starts the keyboard cursor on what is already chosen", async () => {
    const user = userEvent.setup();
    const onChange = renderSelect(vi.fn(), "dark");

    screen.getByRole("combobox", { name: "Tema" }).focus();
    await user.keyboard("{ArrowDown}{Enter}");

    expect(onChange).toHaveBeenCalledWith("dark");
  });

  it("goes to the ends with Home and End", async () => {
    const user = userEvent.setup();
    const onChange = renderSelect(vi.fn(), "light");

    await user.click(screen.getByRole("combobox", { name: "Tema" }));
    await user.keyboard("{End}{Enter}");

    expect(onChange).toHaveBeenCalledWith("dark");
  });

  it("closes on Escape without choosing", async () => {
    const user = userEvent.setup();
    const onChange = renderSelect();

    await user.click(screen.getByRole("combobox", { name: "Tema" }));
    await user.keyboard("{Escape}");

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Tema" })).toHaveFocus();
  });

  it("closes when something outside is pressed", async () => {
    const user = userEvent.setup();
    renderSelect();

    await user.click(screen.getByRole("combobox", { name: "Tema" }));
    await user.click(document.body);

    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("marks what is chosen and not merely what the cursor is on", async () => {
    const user = userEvent.setup();
    renderSelect(vi.fn(), "dark");

    await user.click(screen.getByRole("combobox", { name: "Tema" }));

    expect(screen.getByRole("option", { name: "Oscuro" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("option", { name: "Claro" })).toHaveAttribute("aria-selected", "false");
  });
});
