import { composeStories } from "@storybook/react-vite";
import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as stories from "./CertificateSelect.stories";

const { ChosenPersonal, ChosenOnBehalfOfAnEntity, Unchosen, Searching } = composeStories(stories);

const box = () => screen.getByRole("combobox", { name: "Certificado" });

describe("the closed certificate selector, by state", () => {
  const cases = [
    { Story: Unchosen, shows: ["Elige un certificado"], hides: [] },
    {
      Story: ChosenPersonal,
      shows: ["LOVELACE BYRON ADA", "A título personal · 00000000T"],
      hides: [],
    },
    {
      Story: ChosenOnBehalfOfAnEntity,
      shows: ["Analytical Engines S.L. · B00000000", "Representante · Grace Hopper Brewster"],
      hides: ["00000000T"],
    },
    { Story: Searching, shows: ["Buscando certificados…"], hides: [] },
  ];

  for (const { Story, shows, hides } of cases) {
    it(`shows ${shows.join(" / ")}, labelled «Certificado» and closed`, () => {
      renderWithCatalog(<Story />);

      expect(screen.getByText("Certificado")).toBeVisible();
      expect(box()).toHaveAttribute("aria-expanded", "false");
      for (const each of shows) {
        expect(box()).toHaveTextContent(each);
      }
      for (const each of hides) {
        expect(box()).not.toHaveTextContent(each);
      }
    });
  }
});
