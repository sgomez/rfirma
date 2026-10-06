import { composeStories } from "@storybook/react-vite";
import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as stories from "./SigningProgressDialog.stories";

const { Presign, Sign, Postsign } = composeStories(stories);

describe("SigningProgressDialog", () => {
  it("blocks the window with no way out, and warns about not removing the card", () => {
    renderWithCatalog(<Sign />);

    const dialog = screen.getByRole("dialog", { name: "Firmando el documento…" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(document.querySelector(".rf-scrim")).toBeInTheDocument();
    expect(within(dialog).queryAllByRole("button")).toEqual([]);
    expect(screen.getByText("No retires la tarjeta hasta que termine.")).toBeInTheDocument();
  });

  it("names the three stages in plain language, without the domain term", () => {
    renderWithCatalog(<Presign />);

    expect(screen.getAllByRole("listitem").map((item) => item.textContent)).toEqual([
      "Preparando la firma",
      "Firmando en la tarjeta",
      "Ensamblando el PDF",
    ]);
  });

  it.each([
    { Story: Presign, current: 0 },
    { Story: Sign, current: 1 },
    { Story: Postsign, current: 2 },
  ])(
    "marks only stage $current as the step under way and advances the bar",
    ({ Story, current }) => {
      renderWithCatalog(<Story />);

      screen.getAllByRole("listitem").forEach((item, index) => {
        if (index === current) {
          expect(item).toHaveAttribute("aria-current", "step");
        } else {
          expect(item).not.toHaveAttribute("aria-current");
        }
      });
      const bar = screen.getByRole("progressbar");
      expect(bar).toHaveAttribute("aria-valuenow", String(current + 1));
      expect(bar).toHaveAttribute("aria-valuemin", "0");
      expect(bar).toHaveAttribute("aria-valuemax", "3");
    },
  );

  it("ignores Intro and Escape: there is nothing to accept or to cancel", async () => {
    const user = userEvent.setup();
    renderWithCatalog(<Sign />);

    await user.keyboard("{Enter}{Escape}");

    expect(screen.getByRole("dialog", { name: "Firmando el documento…" })).toBeInTheDocument();
  });
});
