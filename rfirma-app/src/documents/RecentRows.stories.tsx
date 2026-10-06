//! Las historias de los recientes: la sección del estado vacío y las filas con cada situación.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { storyRecents } from "../../.storybook/fixtures/documents";
import { RecentRows, RecentsSection } from "./RecentRows";

const meta = {
  title: "Dominio/Documentos/RecentRows",
  component: RecentsSection,
  parameters: { layout: "centered" },
  decorators: [
    (Story) => (
      <div style={{ width: 420 }}>
        <Story />
      </div>
    ),
  ],
  args: { recents: storyRecents, onSelect: fn(), onClear: fn() },
} satisfies Meta<typeof RecentsSection>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Section: Story = {};

export const SingleRecent: Story = { args: { recents: storyRecents.slice(0, 1) } };

export const Rows: Story = {
  render: () => (
    <RecentRows
      recents={storyRecents}
      openIds={new Set([storyRecents[0]?.id ?? ""])}
      onSelect={fn()}
    />
  ),
};
