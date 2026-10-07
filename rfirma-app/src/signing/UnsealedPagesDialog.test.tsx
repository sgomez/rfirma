import { composeStories } from "@storybook/react-vite";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fn } from "storybook/test";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as stories from "./UnsealedPagesDialog.stories";

const { SeveralPages, OnePage, ManyPages } = composeStories(stories);

describe("UnsealedPagesDialog", () => {
  it.each([
    { Story: SeveralPages, title: "3 páginas se quedarán sin firma visible" },
    { Story: OnePage, title: "Una página se quedará sin firma visible" },
    { Story: ManyPages, title: "12 páginas se quedarán sin firma visible" },
  ])("says «$title» and that the signature stays valid", ({ Story, title }) => {
    renderWithCatalog(<Story />);

    expect(screen.getByRole("dialog", { name: title })).toBeVisible();
    expect(
      screen.getByText(
        "El recuadro no cabe en páginas más pequeñas que aquella donde lo colocaste. " +
          "La firma será válida en todo el documento.",
      ),
    ).toBeInTheDocument();
    expect(screen.queryByText(/recortad/i)).not.toBeInTheDocument();
  });

  it("never names a fallen page, however many fall", () => {
    renderWithCatalog(<ManyPages />);

    expect(screen.queryByText(/\b1\b.*\b2\b.*\b3\b/)).not.toBeInTheDocument();
  });

  it("signs anyway on confirm, and cancels without signing", async () => {
    const user = userEvent.setup();
    const onConfirm = fn();
    const onCancel = fn();
    renderWithCatalog(<SeveralPages onConfirm={onConfirm} onCancel={onCancel} />);

    await user.click(screen.getByRole("button", { name: "Firmar de todos modos" }));
    expect(onConfirm).toHaveBeenCalledOnce();
    expect(onCancel).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(onCancel).toHaveBeenCalledOnce();
  });
});
