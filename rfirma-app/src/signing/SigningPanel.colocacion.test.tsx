import { fireEvent, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { rect, renderLivePanel, renderPanel } from "./SigningPanel.testSupport";
import { DEFAULT_VISIBLE_SIGNATURE } from "./visibleSignature";

describe("SigningPanel · Firma visible, en qué páginas", () => {
  const visible = { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true };

  const signButton = () => screen.getByRole("button", { name: "Firmar como Ada Lovelace" });
  const field = () => screen.getByRole("textbox", { name: "Páginas de la firma visible" });
  const block = () => screen.getByRole("region", { name: "Firma visible" });

  it("chooses one page, several or all in a segmented group, in that order", () => {
    renderPanel({ signature: visible });

    const group = screen.getByRole("radiogroup", { name: "En qué páginas" });
    expect(
      within(group)
        .getAllByRole("radio")
        .map((radio) => radio.parentElement?.textContent),
    ).toEqual(["Una página", "Varias", "Todas"]);
    expect(within(group).getByRole("radio", { name: "Una página" })).toBeChecked();
  });

  it("moves along the segmented group with the arrow keys", async () => {
    const user = userEvent.setup();
    renderLivePanel({ signature: visible });

    screen.getByRole("radio", { name: "Una página" }).focus();
    await user.keyboard("{ArrowRight}");

    expect(screen.getByRole("radio", { name: "Varias" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "Varias" })).toHaveFocus();

    await user.keyboard("{ArrowLeft}{ArrowLeft}");

    expect(screen.getByRole("radio", { name: "Todas" })).toBeChecked();
  });

  it("says the page it is on under «one page», with nothing to press while looking at it", () => {
    renderPanel({ signature: visible, viewedPage: 3 });

    expect(within(block()).getByText("En la página 3")).toBeInTheDocument();
    expect(within(block()).queryByRole("button", { name: /aquí/ })).not.toBeInTheDocument();
  });

  it("moves it to the page in view under «one page» when looking at another", async () => {
    const user = userEvent.setup();
    const onSeal = vi.fn();
    renderPanel({ signature: visible, viewedPage: 7, onSeal });

    expect(within(block()).getByText("En la página 3")).toBeInTheDocument();
    await user.click(within(block()).getByRole("button", { name: "Ponerla aquí" }));

    expect(onSeal).toHaveBeenCalled();
  });

  it("offers to take it off the page in view under «several» when that page has it", async () => {
    const user = userEvent.setup();
    const onUnseal = vi.fn();
    const pages = { only: [3, 10] };
    renderPanel({
      signature: visible,
      pageChoice: "these",
      placement: { rect, pages },
      pageSets: { single: 3, these: pages },
      viewedPage: 10,
      onUnseal,
    });

    await user.click(within(block()).getByRole("button", { name: "Quitarla de aquí" }));

    expect(onUnseal).toHaveBeenCalled();
  });

  it("offers to put it on the page in view under «several» when that page lacks it", async () => {
    const user = userEvent.setup();
    const onSeal = vi.fn();
    const pages = { only: [3, 10] };
    renderPanel({
      signature: visible,
      pageChoice: "these",
      placement: { rect, pages },
      pageSets: { single: 3, these: pages },
      viewedPage: 5,
      onSeal,
    });

    await user.click(within(block()).getByRole("button", { name: "Ponerla aquí" }));

    expect(onSeal).toHaveBeenCalled();
  });

  it("shows nothing under «all»", () => {
    renderPanel({
      signature: visible,
      placement: { rect, pages: "all" },
      pageChoice: "all",
      viewedPage: 3,
    });

    expect(within(block()).queryByRole("button", { name: /aquí/ })).not.toBeInTheDocument();
    expect(within(block()).queryByRole("textbox")).not.toBeInTheDocument();
    expect(within(block()).queryByText(/En la página/)).not.toBeInTheDocument();
  });

  it("signs with the switch on even before anything is placed, and never asks to place it", () => {
    renderPanel({ signature: visible, placement: null });

    expect(signButton()).toBeEnabled();
    expect(screen.queryByText(/Coloca la firma/)).not.toBeInTheDocument();
  });

  it("signs invisibly with the switch off, and shows none of the page choices", () => {
    renderPanel({ signature: { ...visible, enabled: false }, placement: null });

    expect(signButton()).toBeEnabled();
    expect(screen.queryByRole("radiogroup")).not.toBeInTheDocument();
  });

  it("does not lose the placement when the switch goes off and on again", () => {
    const onChoosePages = vi.fn();
    const { show } = renderPanel({ signature: visible, onChoosePages });

    show({ signature: { ...visible, enabled: false }, onChoosePages });
    show({ signature: visible, onChoosePages });

    expect(onChoosePages).not.toHaveBeenCalled();
    expect(screen.getByText("En la página 3")).toBeInTheDocument();
  });

  it("places what the field says, in the everyday print format", () => {
    const { chosen } = renderLivePanel({ signature: visible, pageChoice: "these" });

    fireEvent.change(field(), { target: { value: "1,2-3,10-20" } });

    expect(chosen.at(-1)).toEqual({
      only: [1, 2, 3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
    });
    expect(field()).toHaveValue("1,2-3,10-20");
  });

  it("neither echoes the pages nor explains the repeated box", () => {
    const pages = { only: [1, 2, 3, 10, 11, 12, 13, 14] };
    renderPanel({
      signature: visible,
      pageChoice: "these",
      placement: { rect, pages },
      pageSets: { single: 1, these: pages },
    });

    expect(within(block()).queryByText(/Se sellará/)).not.toBeInTheDocument();
    expect(within(block()).queryByText(/mismo recuadro/)).not.toBeInTheDocument();
  });

  it("never says «sello» nor «sellar» in the block", () => {
    const pages = { only: [3, 10] };
    renderPanel({
      signature: visible,
      pageChoice: "these",
      placement: { rect, pages },
      pageSets: { single: 3, these: pages },
    });
    fireEvent.change(field(), { target: { value: "" } });

    expect(block().textContent).not.toMatch(/sell/i);
    expect(field().getAttribute("aria-label")).not.toMatch(/sell/i);
  });

  it.each([
    ["3-1", "«3-1» va al revés"],
    ["0", "No hay página 0"],
    ["99", "Solo hay 27 páginas"],
    ["1;2", "Separa las páginas con comas"],
  ])("turns the sign button off and says why, short, for %s", (typed, said) => {
    const { chosen } = renderLivePanel({ signature: visible, pageChoice: "these" });

    fireEvent.change(field(), { target: { value: typed } });

    expect(within(block()).getByText(said)).toBeInTheDocument();
    expect(field()).toHaveAttribute("aria-invalid", "true");
    expect(within(block()).queryByRole("button", { name: /aquí/ })).not.toBeInTheDocument();
    expect(signButton()).toBeDisabled();
    expect(chosen).toEqual([]);
  });

  it("rewrites the field when a page is taken off from the viewer", () => {
    const props = { signature: visible, pageChoice: "these" as const };
    const placed = { only: [3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20] };
    const { show } = renderPanel({
      ...props,
      placement: { rect, pages: placed },
      pageSets: { single: 3, these: placed },
    });

    expect(field()).toHaveValue("3,10-20");

    const rest = { only: [3, 10, 11, 13, 14, 15, 16, 17, 18, 19, 20] };
    show({ ...props, placement: { rect, pages: rest }, pageSets: { single: 3, these: rest } });

    expect(field()).toHaveValue("3,10-11,13-20");
  });

  it("asks for the option and does not decide the set that goes with it", async () => {
    const user = userEvent.setup();
    const onChoosePages = vi.fn();
    const onChangePageChoice = vi.fn();
    renderPanel({
      signature: visible,
      pageChoice: "these",
      placement: { rect, pages: { only: [3, 10, 11] } },
      pageSets: { single: 3, these: { only: [3, 10, 11] } },
      onChoosePages,
      onChangePageChoice,
    });

    await user.click(screen.getByRole("radio", { name: "Una página" }));

    expect(onChangePageChoice).toHaveBeenCalledWith("single");
    expect(onChoosePages).not.toHaveBeenCalled();
  });

  it("keeps the page of the box on the round trip one page, all and one page again", async () => {
    const user = userEvent.setup();
    renderLivePanel({ signature: visible });

    await user.click(screen.getByRole("radio", { name: "Todas" }));

    expect(screen.queryByText("En la página 3")).not.toBeInTheDocument();

    await user.click(screen.getByRole("radio", { name: "Una página" }));

    expect(screen.getByText("En la página 3")).toBeInTheDocument();
  });

  it("says the empty field instead of taking the box away with it", () => {
    const { chosen } = renderLivePanel({ signature: visible, pageChoice: "these" });

    fireEvent.change(field(), { target: { value: "" } });

    expect(chosen).toEqual([]);
    expect(within(block()).getByText("Escribe las páginas")).toBeInTheDocument();
    expect(signButton()).toBeDisabled();

    fireEvent.change(field(), { target: { value: "5" } });

    expect(chosen.at(-1)).toEqual({ only: [5] });
  });
});
