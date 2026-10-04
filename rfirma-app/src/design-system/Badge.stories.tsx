//! La historia de `Badge`, con control para su variante.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { Badge } from "./Badge";

const meta = {
  title: "Sistema de diseño/Badge",
  component: Badge,
  args: { children: "v1.2.3" },
  argTypes: { variant: { control: "select", options: [undefined, "primary"] } },
} satisfies Meta<typeof Badge>;

export default meta;

export const Neutral: StoryObj<typeof meta> = {};
export const Primary: StoryObj<typeof meta> = { args: { variant: "primary" } };
