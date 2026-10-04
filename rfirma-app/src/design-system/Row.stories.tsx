//! La historia de `Row`, con control para su hueco.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { Row } from "./Row";

const meta = {
  title: "Sistema de diseño/Row",
  component: Row,
  argTypes: { gap: { control: "select", options: [undefined, "xs", "sm"] } },
  render: (args) => (
    <Row {...args}>
      <span>Primero</span>
      <span>Segundo</span>
      <span>Tercero</span>
    </Row>
  ),
} satisfies Meta<typeof Row>;

export default meta;

export const Default: StoryObj<typeof meta> = {};
export const SmallGap: StoryObj<typeof meta> = { args: { gap: "xs" } };
export const MediumGap: StoryObj<typeof meta> = { args: { gap: "sm" } };
