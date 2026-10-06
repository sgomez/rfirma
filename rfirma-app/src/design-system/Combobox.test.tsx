import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Combobox, type ComboboxOption } from "./Combobox";

interface City {
  name: string;
}

const city = (name: string, extra: Partial<ComboboxOption<City>> = {}): ComboboxOption<City> => ({
  id: name,
  item: { name },
  keywords: [name],
  ...extra,
});

const flat = [city("Ávila"), city("Burgos"), city("Cádiz"), city("Lugo")];

function renderCombobox({
  options = flat,
  value = null as string | null,
  onChange = vi.fn(),
  defaultOpen = false,
} = {}) {
  render(
    <Combobox
      label="Ciudad"
      options={options}
      value={value}
      onChange={onChange}
      renderOption={(item) => <span>{item.name}</span>}
      searchPlaceholder="Busca una ciudad"
      emptyMessage="Ninguna ciudad coincide"
      countLabel={(shown, total) => `${shown} de ${total}`}
      defaultOpen={defaultOpen}
    >
      {value ?? "Elige una"}
    </Combobox>,
  );
  return onChange;
}

async function open() {
  const user = userEvent.setup();
  await user.click(screen.getByRole("combobox", { name: "Ciudad" }));
  return user;
}

const search = () => screen.getByRole("combobox", { name: "Ciudad" });
const activeOption = () =>
  document.getElementById(search().getAttribute("aria-activedescendant") ?? "");

/**
 * **Grada A**: un componente y su teclado.
 */
describe("Combobox", () => {
  it("shows a closed box that is a combobox and no list", () => {
    renderCombobox();

    const box = screen.getByRole("combobox", { name: "Ciudad" });
    expect(box).toHaveAttribute("aria-expanded", "false");
    expect(box).toHaveTextContent("Elige una");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("opens into a search box with the focus on it and the list below", async () => {
    renderCombobox();
    await open();

    expect(search()).toHaveFocus();
    expect(search()).toHaveAttribute("aria-expanded", "true");
    expect(search()).toHaveAttribute("placeholder", "Busca una ciudad");
    expect(within(screen.getByRole("listbox")).getAllByRole("option")).toHaveLength(4);
  });

  it("starts open with defaultOpen", () => {
    renderCombobox({ defaultOpen: true });

    expect(screen.getByRole("listbox")).toBeInTheDocument();
  });

  it("moves the cursor with the arrows and points at it with aria-activedescendant", async () => {
    renderCombobox();
    const user = await open();

    expect(activeOption()).toHaveTextContent("Ávila");
    await user.keyboard("{ArrowDown}{ArrowDown}");
    expect(activeOption()).toHaveTextContent("Cádiz");
    await user.keyboard("{ArrowUp}");
    expect(activeOption()).toHaveTextContent("Burgos");
    expect(screen.getByRole("listbox")).toContainElement(activeOption());
  });

  it("goes to the ends with Home and End", async () => {
    renderCombobox();
    const user = await open();

    await user.keyboard("{End}");
    expect(activeOption()).toHaveTextContent("Lugo");
    await user.keyboard("{Home}");
    expect(activeOption()).toHaveTextContent("Ávila");
  });

  it("starts the cursor on what is already chosen", async () => {
    renderCombobox({ value: "Cádiz" });
    await open();

    expect(activeOption()).toHaveTextContent("Cádiz");
    expect(screen.getByRole("option", { name: "Cádiz" })).toHaveAttribute("aria-selected", "true");
  });

  it("chooses with Enter and closes", async () => {
    const onChange = renderCombobox();
    const user = await open();

    await user.keyboard("{ArrowDown}{Enter}");

    expect(onChange).toHaveBeenCalledWith({ name: "Burgos" });
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("chooses with the pointer", async () => {
    const onChange = renderCombobox();
    const user = await open();

    await user.click(screen.getByRole("option", { name: "Lugo" }));

    expect(onChange).toHaveBeenCalledWith({ name: "Lugo" });
  });

  it("filters ignoring diacritics and case, and counts what it shows", async () => {
    renderCombobox();
    const user = await open();

    await user.keyboard("AVI");

    expect(screen.getAllByRole("option").map((one) => one.textContent)).toEqual(["Ávila"]);
    expect(screen.getByText("1 de 4")).toBeInTheDocument();
  });

  it("matches any of the option keywords", async () => {
    renderCombobox({ options: [city("Lugo", { keywords: ["Lugo", "Galicia"] }), city("Burgos")] });
    const user = await open();

    await user.keyboard("galicia");

    expect(screen.getAllByRole("option").map((one) => one.textContent)).toEqual(["Lugo"]);
  });

  it("does not count while nothing is typed", async () => {
    renderCombobox();
    await open();

    expect(screen.queryByText("4 de 4")).not.toBeInTheDocument();
  });

  it("says so when nothing matches", async () => {
    renderCombobox();
    const user = await open();

    await user.keyboard("zzz");

    expect(screen.queryAllByRole("option")).toHaveLength(0);
    expect(screen.getByText("Ninguna ciudad coincide")).toBeInTheDocument();
    expect(search()).not.toHaveAttribute("aria-activedescendant");
  });

  it("never chooses a disabled option", async () => {
    const onChange = renderCombobox({
      options: [city("Ávila", { disabled: true }), city("Burgos")],
    });
    const user = await open();

    expect(screen.getByRole("option", { name: "Ávila" })).toHaveAttribute("aria-disabled", "true");
    await user.keyboard("{Enter}");
    await user.click(screen.getByRole("option", { name: "Ávila" }));

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByRole("listbox")).toBeInTheDocument();
  });

  it("paints a header per group when there is more than one", async () => {
    renderCombobox({
      options: [
        city("Ávila", { group: "Castilla y León" }),
        city("Burgos", { group: "Castilla y León" }),
        city("Lugo", { group: "Galicia" }),
      ],
    });
    const user = await open();

    const groups = screen.getAllByRole("group");
    expect(groups.map((group) => group.getAttribute("aria-label"))).toEqual([
      "Castilla y León",
      "Galicia",
    ]);
    expect(within(groups[1] as HTMLElement).getByRole("option", { name: "Lugo" })).toBeVisible();
    await user.keyboard("{End}");
    expect(activeOption()).toHaveTextContent("Lugo");
  });

  it("paints no header when every option is in the same group", async () => {
    renderCombobox({
      options: [
        city("Ávila", { group: "Castilla y León" }),
        city("Burgos", { group: "Castilla y León" }),
      ],
    });
    await open();

    expect(screen.queryByRole("group")).not.toBeInTheDocument();
    expect(screen.queryByText("Castilla y León")).not.toBeInTheDocument();
  });

  it("closes on Escape without choosing and gives the focus back to the box", async () => {
    const onChange = renderCombobox();
    const user = await open();

    await user.keyboard("{Escape}");

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Ciudad" })).toHaveFocus();
  });

  it("does not open while disabled", async () => {
    const user = userEvent.setup();
    render(
      <Combobox
        label="Ciudad"
        options={flat}
        value={null}
        onChange={vi.fn()}
        renderOption={(item) => item.name}
        searchPlaceholder="Busca"
        emptyMessage="Nada"
        countLabel={(shown, total) => `${shown} de ${total}`}
        disabled
      >
        Elige una
      </Combobox>,
    );

    await user.click(screen.getByRole("combobox", { name: "Ciudad" }));

    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });
});
