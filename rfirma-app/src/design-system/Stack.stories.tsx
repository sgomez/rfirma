//! La historia de `Stack`, con control para su hueco.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { Stack } from "./Stack";

const meta = {
  title: "Primitivos/Stack",
  component: Stack,
  argTypes: { gap: { control: "select", options: [undefined, "xs", "md"] } },
  render: (args) => (
    <Stack {...args}>
      <span>Primero</span>
      <span>Segundo</span>
      <span>Tercero</span>
    </Stack>
  ),
} satisfies Meta<typeof Stack>;

export default meta;

export const Default: StoryObj<typeof meta> = {};
export const GapXs: StoryObj<typeof meta> = { args: { gap: "xs" } };
export const GapMd: StoryObj<typeof meta> = { args: { gap: "md" } };
