import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { createI18n } from "../i18n/i18n";
import { LanguageProvider } from "../i18n/LanguageProvider";
import { inMemoryLanguagePreference } from "../i18n/preference";
import { PlacementBlock } from "./PlacementBlock";
import type { Placement, UserSpaceRect } from "./pageSets";
import { type PlacementState, type StandardRectOn, usePlacement } from "./usePlacement";

const PAGE_COUNT = 27;

const RECT: UserSpaceRect = { x0: 100, y0: 100, x1: 300, y1: 180 };

const standardRectOn: StandardRectOn = async (page) => ({
  x0: page,
  y0: page,
  x1: page + 100,
  y1: page + 40,
});

const i18n = createI18n("es");

function WithCatalog({ children }: { children: ReactNode }) {
  return (
    <LanguageProvider i18n={i18n} preference={inMemoryLanguagePreference("es")}>
      {children}
    </LanguageProvider>
  );
}

function renderBlock(saved: Placement | null) {
  const onChange = vi.fn();
  const document = { placement: saved, pageCount: PAGE_COUNT };
  const state: { current: PlacementState | null } = { current: null };
  function Live() {
    const placement = usePlacement({ document, standardRectOn, onChange });
    state.current = placement;
    return <PlacementBlock state={placement} />;
  }
  render(<Live />, { wrapper: WithCatalog });
  const latest = () => {
    if (state.current === null) throw new Error("el bloque no se ha pintado");
    return state.current;
  };
  return { onChange, latest };
}

const onPageThree: Placement = { rect: RECT, pages: { only: [3] } };
const onThreeAndTen: Placement = { rect: RECT, pages: { only: [3, 10] } };

const field = () => screen.getByRole("textbox", { name: "Páginas de la firma visible" });
const radio = (name: string) => screen.getByRole("radio", { name });

describe("PlacementBlock", () => {
  it("moves along the segmented group with the arrow keys", async () => {
    const user = userEvent.setup();
    renderBlock(onPageThree);

    radio("Una página").focus();
    await user.keyboard("{ArrowRight}");

    expect(radio("Varias")).toBeChecked();
    expect(radio("Varias")).toHaveFocus();

    await user.keyboard("{ArrowLeft}{ArrowLeft}");

    expect(radio("Todas")).toBeChecked();
  });

  it("moves the box to the page in view under «one page» instead of adding it", async () => {
    const user = userEvent.setup();
    const { latest } = renderBlock(onPageThree);
    act(() => latest().viewPage(7));

    expect(screen.getByText("En la página 3")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Ponerla aquí" }));

    expect(screen.getByText("En la página 7")).toBeInTheDocument();
    expect(latest().placement?.pages).toEqual({ only: [7] });
  });

  it("offers no page button under «one page» on the page that has it", () => {
    renderBlock(onPageThree);

    expect(screen.queryByRole("button", { name: /aquí/ })).not.toBeInTheDocument();
  });

  it("takes the page in view off under «several» when that page has it", async () => {
    const user = userEvent.setup();
    const { latest } = renderBlock(onThreeAndTen);
    act(() => latest().viewPage(10));

    await user.click(screen.getByRole("button", { name: "Quitarla de aquí" }));

    expect(field()).toHaveValue("3");
  });

  it("puts the page in view on under «several» when that page lacks it", async () => {
    const user = userEvent.setup();
    const { latest } = renderBlock(onThreeAndTen);
    act(() => latest().viewPage(5));

    await user.click(screen.getByRole("button", { name: "Ponerla aquí" }));

    expect(field()).toHaveValue("3,5,10");
  });

  it("shows nothing under «all pages»", async () => {
    const user = userEvent.setup();
    renderBlock(onPageThree);

    await user.click(radio("Todas"));

    expect(screen.queryByText("En la página 3")).not.toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("keeps the page of the box on the round trip one page, all and one page again", async () => {
    const user = userEvent.setup();
    renderBlock(onPageThree);

    await user.click(radio("Todas"));
    await user.click(radio("Una página"));

    expect(screen.getByText("En la página 3")).toBeInTheDocument();
  });

  it("starts «several» for the first time with the page of «one page»", async () => {
    const user = userEvent.setup();
    renderBlock(onPageThree);

    await user.click(radio("Varias"));

    expect(field()).toHaveValue("3");
  });

  it("places what the field says, in the everyday print format", () => {
    const { latest } = renderBlock(onThreeAndTen);

    fireEvent.change(field(), { target: { value: "1,2-3,10-20" } });

    expect(latest().placement?.pages).toEqual({
      only: [1, 2, 3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
    });
    expect(field()).toHaveValue("1,2-3,10-20");
  });

  it("does not rewrite what is being typed", async () => {
    const user = userEvent.setup();
    const { latest } = renderBlock(onThreeAndTen);

    await user.clear(field());
    await user.type(field(), "1,3");

    expect(field()).toHaveValue("1,3");
    expect(latest().placement?.pages).toEqual({ only: [1, 3] });
  });

  it.each([
    ["3-1", "«3-1» va al revés"],
    ["0", "No hay página 0"],
    ["99", "Solo hay 27 páginas"],
    ["1;2", "Separa las páginas con comas"],
  ])("says why, short, and applies nothing of %s", (typed, said) => {
    const { onChange, latest } = renderBlock(onThreeAndTen);

    fireEvent.change(field(), { target: { value: typed } });

    expect(screen.getByText(said)).toBeInTheDocument();
    expect(field()).toHaveAttribute("aria-invalid", "true");
    expect(screen.queryByRole("button", { name: /aquí/ })).not.toBeInTheDocument();
    expect(onChange).not.toHaveBeenCalled();
    expect(latest().placement?.pages).toEqual({ only: [3, 10] });
  });

  it("says the empty field instead of taking the box away with it", () => {
    const { onChange, latest } = renderBlock(onThreeAndTen);

    fireEvent.change(field(), { target: { value: "" } });

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByText("Escribe las páginas")).toBeInTheDocument();
    expect(latest().rangeError).toEqual({ kind: "empty" });

    fireEvent.change(field(), { target: { value: "5" } });

    expect(latest().placement?.pages).toEqual({ only: [5] });
  });

  it("rewrites the field when a page is taken off from the viewer", async () => {
    const { latest } = renderBlock({
      rect: RECT,
      pages: { only: [3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20] },
    });
    expect(field()).toHaveValue("3,10-20");

    act(() => latest().unsealPage(12));

    await waitFor(() => expect(field()).toHaveValue("3,10-11,13-20"));
  });
});
