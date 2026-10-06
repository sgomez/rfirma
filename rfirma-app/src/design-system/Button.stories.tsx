//! La historia de `Button`, con control para su variante y para si está deshabilitado.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { Button } from "./Button";

const meta = {
  title: "Primitivos/Button",
  component: Button,
  args: { children: "Firmar" },
  argTypes: {
    variant: { control: "select", options: [undefined, "primary", "secondary", "ghost"] },
    disabled: { control: "boolean" },
  },
} satisfies Meta<typeof Button>;

export default meta;

export const Primary: StoryObj<typeof meta> = { args: { variant: "primary" } };
export const Secondary: StoryObj<typeof meta> = { args: { variant: "secondary" } };
export const Ghost: StoryObj<typeof meta> = { args: { variant: "ghost" } };
export const Plain: StoryObj<typeof meta> = {};
