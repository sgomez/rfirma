import { composeStories } from "@storybook/react-vite";
import { render, waitFor } from "@testing-library/react";
import axe from "axe-core";
import type { ComponentType } from "react";
import { describe, expect, it } from "vitest";
import { CatalogProvider } from "./testing/render";

const STRUCTURAL_RULES = { "color-contrast": { enabled: false } };

type RuleSwitch = { id: string; enabled: boolean };

function rulesOf(story: { parameters?: Record<string, unknown> }) {
  const switches =
    (story.parameters?.a11y as { config?: { rules?: RuleSwitch[] } } | undefined)?.config?.rules ??
    [];
  return {
    ...STRUCTURAL_RULES,
    ...Object.fromEntries(switches.map(({ id, enabled }) => [id, { enabled }])),
  };
}

const modules = import.meta.glob<Parameters<typeof composeStories>[0]>("./**/*.stories.tsx", {
  eager: true,
});

describe("stories", () => {
  it("finds the story files", () => {
    expect(Object.keys(modules).length).toBeGreaterThan(0);
  });

  for (const [file, module] of Object.entries(modules)) {
    for (const [name, Story] of Object.entries(
      composeStories(module) as Record<
        string,
        ComponentType & { parameters?: Record<string, unknown> }
      >,
    )) {
      it(`${file} · ${name} renders without errors or structural accessibility failures`, async () => {
        const { container } = render(
          <CatalogProvider language="es">
            <Story />
          </CatalogProvider>,
        );

        await waitFor(() => expect(container).not.toBeEmptyDOMElement());
        const { violations } = await axe.run(container, { rules: rulesOf(Story) });
        expect(violations.map(({ id, nodes }) => `${id}: ${nodes[0]?.html}`)).toEqual([]);
      });
    }
  }
});
