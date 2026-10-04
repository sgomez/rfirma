import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef, useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { Menu, MenuItem } from "./Menu";

function Harness({ onPick = () => {} }: { onPick?: (entry: string) => void }) {
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const pick = (entry: string) => () => {
    setOpen(false);
    onPick(entry);
  };
  return (
    <>
      <button type="button">fuera</button>
      <div ref={anchor}>
        <button type="button" ref={trigger} onClick={() => setOpen((was) => !was)}>
          abrir
        </button>
        <Menu
          open={open}
          onClose={() => setOpen(false)}
          anchorRef={anchor}
          returnFocusRef={trigger}
          aria-label="Acciones"
        >
          <MenuItem onClick={pick("primera")}>
            <strong>Primera</strong> <em>con detalle</em>
          </MenuItem>
          <hr />
          <MenuItem onClick={pick("segunda")}>Segunda</MenuItem>
          <MenuItem onClick={pick("tercera")}>Tercera</MenuItem>
        </Menu>
      </div>
    </>
  );
}

async function openMenu(ui = <Harness />) {
  const user = userEvent.setup();
  render(ui);
  await user.click(screen.getByRole("button", { name: "abrir" }));
  return user;
}

const item = (name: string) => screen.getByRole("menuitem", { name: new RegExp(name) });

describe("Menu", () => {
  it("opens as a menu with the focus on its first item", async () => {
    await openMenu();

    expect(screen.getByRole("menu", { name: "Acciones" })).toBeInTheDocument();
    expect(item("Primera")).toHaveFocus();
  });

  it("keeps its items out of the tab order", async () => {
    await openMenu();

    for (const entry of screen.getAllByRole("menuitem")) {
      expect(entry).toHaveAttribute("tabindex", "-1");
    }
  });

  it("moves down and up with the arrows, skipping the separator and wrapping at both ends", async () => {
    const user = await openMenu();

    await user.keyboard("{ArrowDown}");
    expect(item("Segunda")).toHaveFocus();
    await user.keyboard("{ArrowDown}{ArrowDown}");
    expect(item("Primera")).toHaveFocus();
    await user.keyboard("{ArrowUp}");
    expect(item("Tercera")).toHaveFocus();
  });

  it("jumps to the last item with End and to the first with Home", async () => {
    const user = await openMenu();

    await user.keyboard("{End}");
    expect(item("Tercera")).toHaveFocus();
    await user.keyboard("{Home}");
    expect(item("Primera")).toHaveFocus();
  });

  it("closes on Escape and gives the focus back to its button", async () => {
    const user = await openMenu();
    await user.keyboard("{ArrowDown}");

    await user.keyboard("{Escape}");

    expect(screen.queryByRole("menu")).toBeNull();
    expect(screen.getByRole("button", { name: "abrir" })).toHaveFocus();
  });

  it("closes on Tab from inside", async () => {
    await openMenu();

    fireEvent.keyDown(item("Primera"), { key: "Tab" });

    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("closes on a press outside", async () => {
    await openMenu();

    fireEvent.pointerDown(screen.getByRole("button", { name: "fuera" }));

    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("runs the chosen item from the keyboard", async () => {
    const onPick = vi.fn();
    const user = await openMenu(<Harness onPick={onPick} />);

    await user.keyboard("{ArrowDown}{Enter}");

    expect(onPick).toHaveBeenCalledWith("segunda");
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("carries arbitrary content in an item", async () => {
    await openMenu();

    expect(item("Primera").querySelector("strong")).toHaveTextContent("Primera");
    expect(item("Primera").querySelector("em")).toHaveTextContent("con detalle");
  });
});
