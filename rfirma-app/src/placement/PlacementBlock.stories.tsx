//! Las historias del bloque «Colocación»: una por modo de páginas y una con el error del campo.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { PlacementBlock } from "./PlacementBlock";
import { placementStateOf } from "./testing/fixtures";

const RECT = { x0: 100, y0: 100, x1: 300, y1: 180 };

const meta = {
  title: "Flujos/Firma/PlacementBlock",
  component: PlacementBlock,
  parameters: { layout: "centered" },
  decorators: [
    (Story) => (
      <section className="panel__placement" style={{ width: 332 }}>
        <Story />
      </section>
    ),
  ],
} satisfies Meta<typeof PlacementBlock>;

export default meta;

type Story = StoryObj<typeof meta>;

export const SinglePage: Story = {
  args: {
    state: placementStateOf({ rect: RECT, sets: { single: 3, these: null }, pageCount: 27 }),
  },
};

export const SinglePageNotSealedYet: Story = {
  args: { state: placementStateOf({ viewedPage: 3, pageCount: 27 }) },
};

export const SeveralPages: Story = {
  args: {
    state: placementStateOf({
      rect: RECT,
      sets: { single: null, these: { only: [1, 3, 4, 5] } },
      mode: "these",
      pageCount: 27,
    }),
  },
};

export const SeveralPagesWithFieldError: Story = {
  args: {
    state: {
      ...placementStateOf({
        rect: RECT,
        sets: { single: null, these: { only: [1, 3] } },
        mode: "these",
        pageCount: 27,
      }),
      pagesText: "1-99",
      rangeError: { kind: "beyond", page: 99, pageCount: 27 },
      pageAction: null,
    },
  },
};

export const AllPages: Story = {
  args: { state: placementStateOf({ rect: RECT, mode: "all", pageCount: 27 }) },
};
