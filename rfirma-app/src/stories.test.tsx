import { composeStories } from "@storybook/react-vite";
import { render } from "@testing-library/react";
import type { ComponentType } from "react";
import { describe, expect, it } from "vitest";
import { CatalogProvider } from "./testing/render";

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
      it(`${file} · ${name} renders without errors`, () => {
        const { container } = render(
          <CatalogProvider language="es">
            <Story />
          </CatalogProvider>,
        );

        expect(container).not.toBeEmptyDOMElement();
      });
    }
  }
});
