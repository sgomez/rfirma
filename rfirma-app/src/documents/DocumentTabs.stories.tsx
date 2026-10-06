//! Las historias de las pestañas de documentos: sin ninguna, con varias, con la firma en curso y sin el botón de abrir.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { DocumentTabs } from "./DocumentTabs";
import { manyStoryTabs, storyRecents, storyTabs } from "./testing/fixtures";

const meta = {
  title: "Dominio/Documentos/DocumentTabs",
  component: DocumentTabs,
  parameters: {
    layout: "fullscreen",
    // Cada pestaña lleva su botón de cerrar junto al `tab`, y axe no admite ese hermano dentro del `tablist`.
    a11y: { config: { rules: [{ id: "aria-required-children", enabled: false }] } },
  },
  decorators: [
    (Story) => (
      <div
        style={{
          display: "flex",
          height: 44,
          background: "var(--rf-bg)",
          borderBottom: "1px solid var(--rf-border-subtle)",
        }}
      >
        <Story />
      </div>
    ),
  ],
  args: {
    tabs: storyTabs,
    activeId: storyTabs[0]?.id ?? null,
    recents: storyRecents,
    onActivate: fn(),
    onClose: fn(),
    onOpen: fn(),
    onSelectRecent: fn(),
    onClearRecents: fn(),
  },
} satisfies Meta<typeof DocumentTabs>;

export default meta;

type Story = StoryObj<typeof meta>;

export const NoTabs: Story = { args: { tabs: [], activeId: null } };

export const Several: Story = {};

export const SignedTabActive: Story = { args: { activeId: storyTabs[1]?.id ?? null } };

export const SigningLocked: Story = { args: { signingLocked: true } };

export const WithoutOpenButton: Story = { args: { withOpenButton: false } };

export const NoRecents: Story = { args: { recents: [] } };

export const ManyTabs: Story = {
  args: { tabs: manyStoryTabs, activeId: manyStoryTabs[0]?.id ?? null },
};
