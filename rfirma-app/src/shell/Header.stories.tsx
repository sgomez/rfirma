//! Las historias de la cabecera: el menú en la barra, el botón de aviso y la variante de Linux, solo con pestañas.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { storyRecents, storyTabs } from "../../.storybook/fixtures/documents";
import { DocumentTabs } from "../documents/DocumentTabs";
import { Header } from "./Header";

const tabs = (
  <DocumentTabs
    tabs={storyTabs}
    activeId={storyTabs[0]?.id ?? null}
    recents={storyRecents}
    onActivate={fn()}
    onClose={fn()}
    onOpen={fn()}
    onSelectRecent={fn()}
    onClearRecents={fn()}
  />
);

const meta = {
  title: "Ventana principal/2 · Cabecera",
  component: Header,
  parameters: {
    layout: "fullscreen",
    // Las pestañas de la cabecera son las mismas de `DocumentTabs`, con el mismo hermano dentro del `tablist`.
    a11y: { config: { rules: [{ id: "aria-required-children", enabled: false }] } },
  },
  args: {
    menuAnchor: "header",
    onOpenStatus: fn(),
    onOpenPreferences: fn(),
    onOpenHelp: fn(),
    onOpenAbout: fn(),
  },
} satisfies Meta<typeof Header>;

export default meta;

type Story = StoryObj<typeof meta>;

export const WithoutDocuments: Story = {};

export const WithDocuments: Story = { args: { documents: tabs } };

export const WithAttention: Story = { args: { documents: tabs, hasAttention: true } };

export const NativeTitlebarWithDocuments: Story = {
  args: { menuAnchor: "titlebar", documents: tabs },
};
