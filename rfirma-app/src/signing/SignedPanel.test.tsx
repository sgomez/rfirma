import { composeStories } from "@storybook/react-vite";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fn } from "storybook/test";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as stories from "./SignedPanel.stories";

const { JustSigned } = composeStories(stories);

// Lo que se ve en cada estado, en `SignedPanel.presentation.test.tsx`.
describe("SignedPanel", () => {
  it.each([
    ["Cambiar", "onChangeDestination"],
    ["Abrir el PDF", "onOpenDocument"],
    ["Abrir la carpeta", "onOpenFolder"],
    ["Firmar", "onSign"],
  ] as const)("calls %s once when pressed", async (name, handler) => {
    const spy = fn();
    renderWithCatalog(<JustSigned {...{ [handler]: spy }} />);

    await userEvent.click(screen.getByRole("button", { name }));

    expect(spy).toHaveBeenCalledOnce();
  });
});
