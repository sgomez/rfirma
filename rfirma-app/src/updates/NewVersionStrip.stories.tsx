//! Las historias de la franja de versión nueva: instalable desde la aplicación y solo con el paso a Acerca de.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { NewVersionStrip } from "./NewVersionStrip";

const meta = {
  title: "Ventana principal/6 · Franja de versión nueva",
  component: NewVersionStrip,
  parameters: { layout: "fullscreen" },
  args: { onOpen: fn(), onDismiss: fn() },
} satisfies Meta<typeof NewVersionStrip>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Installable: Story = { args: { newVersion: { version: "0.4.1", installable: true } } };

export const NotInstallable: Story = {
  args: { newVersion: { version: "0.4.1", installable: false } },
};
