import { composeStories } from "@storybook/react-vite";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fn } from "storybook/test";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as stories from "./Header.stories";

const { WithoutDocuments, WithDocuments, WithAttention, NativeTitlebarWithDocuments } =
  composeStories(stories);

const attentionName = "Estado de rFirma: requiere atención";

describe("the header as it opens", () => {
  const withoutAttention = [
    { name: "without documents", Story: WithoutDocuments, documents: false },
    { name: "with documents", Story: WithDocuments, documents: true },
  ];

  for (const { name, Story, documents } of withoutAttention) {
    it(`carries a closed menu button with an icon, no app name and no attention button, ${name}`, () => {
      renderWithCatalog(<Story />);

      const banner = screen.getByRole("banner");
      const menu = screen.getByRole("button", { name: "Menú" });
      expect(menu.querySelector("svg")).not.toBeNull();
      expect(menu).toHaveTextContent("");
      expect(menu).toHaveAttribute("aria-expanded", "false");
      expect(menu).toHaveClass("header__button");
      expect(menu).not.toHaveClass("header__button--open");
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
      expect(screen.queryByRole("menubar")).not.toBeInTheDocument();
      expect(banner).not.toHaveTextContent("rFirma");
      expect(screen.queryByText("Sin firmar")).not.toBeInTheDocument();
      expect(screen.queryByText("Firmado")).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: attentionName })).not.toBeInTheDocument();

      const tabs = screen.queryByRole("tablist");
      if (documents) {
        expect(banner).toContainElement(tabs);
        expect(tabs?.compareDocumentPosition(menu)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
      } else {
        expect(tabs).not.toBeInTheDocument();
      }
    });
  }

  it("carries only the tabs when the menu lives in the native titlebar", () => {
    renderWithCatalog(<NativeTitlebarWithDocuments />);

    expect(screen.getByRole("banner")).toContainElement(screen.getByRole("tablist"));
    expect(screen.queryByRole("button", { name: "Menú" })).not.toBeInTheDocument();
  });

  it("shows the attention button left of the menu button, with its name and tooltip", () => {
    renderWithCatalog(<WithAttention />);

    const attention = screen.getByRole("button", { name: attentionName });
    expect(attention).toHaveAttribute("title", attentionName);
    const menu = screen.getByRole("button", { name: "Menú" });
    expect(attention.compareDocumentPosition(menu) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });
});

describe("the header menu", () => {
  async function openMenu(ui: React.ReactElement) {
    const user = userEvent.setup();
    renderWithCatalog(ui);
    await user.click(screen.getByRole("button", { name: "Menú" }));
    return user;
  }

  it("opens four entries in two groups, with the external icon only on help", async () => {
    await openMenu(<WithAttention />);

    const items = screen.getAllByRole("menuitem");
    expect(items.map((item) => item.textContent?.trim())).toEqual([
      "Estado de rFirma",
      "Preferencias…",
      "Comentarios y ayuda",
      "Acerca de rFirma",
    ]);
    expect(screen.getByRole("separator")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Menú" })).toHaveClass("header__button--open");
    for (const item of items) {
      expect(item.querySelector(".header__entryIcon")).not.toBeNull();
    }
    expect(items.map((item) => item.querySelector(".header__entryIcon svg") !== null)).toEqual([
      false,
      false,
      true,
      false,
    ]);
  });

  const entries = [
    { entry: "Estado de rFirma", handler: "onOpenStatus" },
    { entry: "Preferencias…", handler: "onOpenPreferences" },
    { entry: /Comentarios y ayuda/, handler: "onOpenHelp" },
    { entry: "Acerca de rFirma", handler: "onOpenAbout" },
  ] as const;

  for (const { entry, handler } of entries) {
    it(`calls ${handler} from its entry and closes the menu`, async () => {
      const spy = fn();
      const user = await openMenu(<WithoutDocuments {...{ [handler]: spy }} />);

      await user.click(screen.getByRole("menuitem", { name: entry }));

      expect(spy).toHaveBeenCalledOnce();
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });
  }

  it("opens the status panel from the attention button", async () => {
    const user = userEvent.setup();
    const onOpenStatus = fn();
    renderWithCatalog(<WithAttention onOpenStatus={onOpenStatus} />);

    await user.click(screen.getByRole("button", { name: attentionName }));

    expect(onOpenStatus).toHaveBeenCalledTimes(1);
  });

  it("closes the open menu with Escape", async () => {
    const user = await openMenu(<WithoutDocuments />);

    await user.keyboard("{Escape}");

    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });
});
