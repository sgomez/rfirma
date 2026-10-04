import { composeStories } from "@storybook/react-vite";
import { render, waitFor } from "@testing-library/react";
import axe from "axe-core";
import type { ComponentType } from "react";
import { describe, expect, it } from "vitest";
import { CatalogProvider } from "./testing/render";

const STRUCTURAL_RULES = { "color-contrast": { enabled: false } };

const modules = import.meta.glob<Parameters<typeof composeStories>[0]>("./**/*.stories.tsx", {
  eager: true,
});

describe("stories", () => {
  it("finds the story files", () => {
    expect(Object.keys(modules).length).toBeGreaterThan(0);
  });

  for (const [file, module] of Object.entries(modules)) {
    for (const [name, Story] of Object.entries(
      composeStories(module) as Record<string, ComponentType>,
    )) {
      it(`${file} · ${name} renders without errors or structural accessibility failures`, async () => {
        const { container } = render(
          <CatalogProvider language="es">
            <Story />
          </CatalogProvider>,
        );

        await waitFor(() => expect(container).not.toBeEmptyDOMElement());
        const { violations } = await axe.run(container, { rules: STRUCTURAL_RULES });
        expect(violations.map(({ id, nodes }) => `${id}: ${nodes[0]?.html}`)).toEqual([]);
      });
    }
  }
});
