import { screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { renderConsoleAt } from "../test/render";

const THE_CHECK = "empty_uri_rejected";

async function theRowOf(id: string) {
  const name = await screen.findByText(id, { selector: ".check-id" });
  return name.closest("[data-check-row]") as HTMLElement;
}

describe("a check row", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("keeps the id of the check outside any button so that it can be selected", async () => {
    renderConsoleAt("/");

    const name = await screen.findByText(THE_CHECK, { selector: ".check-id" });

    expect(name.closest("button")).toBeNull();
  });

  it("unfolds the check on a click that selects nothing", async () => {
    const { user } = renderConsoleAt("/");
    const row = await theRowOf(THE_CHECK);

    await user.click(row);

    expect(row).toHaveAttribute("aria-expanded", "true");
  });

  it("does not unfold the check when the click ends a text selection", async () => {
    const { user } = renderConsoleAt("/");
    const row = await theRowOf(THE_CHECK);
    vi.spyOn(window, "getSelection").mockReturnValue({
      isCollapsed: false,
      toString: () => THE_CHECK,
    } as Selection);

    await user.click(row);

    expect(row).toHaveAttribute("aria-expanded", "false");
  });

  it("copies the id of the check to the clipboard from its own button", async () => {
    const { user } = renderConsoleAt("/");

    await user.click(await screen.findByRole("button", { name: `Copiar ${THE_CHECK}` }));

    expect(await navigator.clipboard.readText()).toBe(THE_CHECK);
  });
});
