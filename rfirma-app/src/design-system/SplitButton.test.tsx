import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { MenuItem } from "./Menu";
import { SplitButton } from "./SplitButton";

function setup(withItems = true) {
  const onAction = vi.fn();
  const onPick = vi.fn();
  render(
    <SplitButton
      onAction={onAction}
      menuLabel="Recientes"
      items={
        withItems ? (
          <>
            <MenuItem onClick={onPick}>Primero</MenuItem>
            <MenuItem>Segundo</MenuItem>
          </>
        ) : undefined
      }
    >
      Abrir PDF
    </SplitButton>,
  );
  return { onAction, onPick, user: userEvent.setup() };
}

const arrow = () => screen.getByRole("button", { name: "Recientes" });

describe("SplitButton", () => {
  it("runs the main action without opening the menu", async () => {
    const { onAction, user } = setup();

    await user.click(screen.getByRole("button", { name: "Abrir PDF" }));

    expect(onAction).toHaveBeenCalledOnce();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("announces the menu on the arrow and keeps it in sync with its state", async () => {
    const { user } = setup();
    expect(arrow()).toHaveAttribute("aria-haspopup", "menu");
    expect(arrow()).toHaveAttribute("aria-expanded", "false");

    await user.click(arrow());

    expect(arrow()).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("menu", { name: "Recientes" })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Primero" })).toHaveFocus();
  });

  it("closes on picking an item", async () => {
    const { onPick, user } = setup();
    await user.click(arrow());

    await user.keyboard("{Enter}");

    expect(onPick).toHaveBeenCalledOnce();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("closes with Escape returning the focus to the arrow", async () => {
    const { user } = setup();
    await user.click(arrow());

    await user.keyboard("{Escape}");

    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(arrow()).toHaveFocus();
  });

  it("paints only the action when there are no items", () => {
    setup(false);

    expect(screen.getByRole("button", { name: "Abrir PDF" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Recientes" })).not.toBeInTheDocument();
  });
});
