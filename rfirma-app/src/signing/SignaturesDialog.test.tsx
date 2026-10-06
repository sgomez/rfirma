import { composeStories } from "@storybook/react-vite";
import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fn } from "storybook/test";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as stories from "./SignaturesDialog.stories";

const { FindingAndSignatures, AllValid } = composeStories(stories);

describe("SignaturesDialog", () => {
  it("titles itself with the format and the count of signatures", () => {
    renderWithCatalog(<AllValid />);

    const dialog = screen.getByRole("dialog", { name: "Firmas del documento" });
    expect(within(dialog).getByText("PAdES · 2 firmas")).toBeVisible();
  });

  it("shows the finding with the validity and the reason of each signature, none of them new", () => {
    renderWithCatalog(<FindingAndSignatures />);

    expect(screen.getByText("Se ha modificado después de la última firma")).toBeVisible();
    expect(screen.getByText("Caducada")).toBeVisible();
    expect(screen.getByText("No válida")).toBeVisible();
    expect(screen.queryByText("Nueva")).not.toBeInTheDocument();
  });

  it("has Cerrar as its only button", async () => {
    const onClose = fn();
    renderWithCatalog(<AllValid onClose={onClose} />);

    expect(screen.getAllByRole("button")).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: "Cerrar" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes with Intro, with the focus already on Cerrar, and with Escape", async () => {
    const user = userEvent.setup();
    const onClose = fn();
    renderWithCatalog(<AllValid onClose={onClose} />);

    expect(screen.getByRole("button", { name: "Cerrar" })).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(onClose).toHaveBeenCalledTimes(1);

    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
