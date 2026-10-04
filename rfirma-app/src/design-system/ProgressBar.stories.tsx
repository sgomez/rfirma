//! La historia de `ProgressBar`, con controles para su valor y su variante.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { ProgressBar } from "./ProgressBar";

const meta = {
  title: "Sistema de diseño/ProgressBar",
  component: ProgressBar,
  decorators: [
    (Story) => (
      <div style={{ width: 320 }}>
        <Story />
      </div>
    ),
  ],
  args: { value: 50, "aria-label": "Progreso" },
  argTypes: { variant: { control: "select", options: [undefined, "framed"] } },
} satisfies Meta<typeof ProgressBar>;

export default meta;

export const Default: StoryObj<typeof meta> = {};
export const Framed: StoryObj<typeof meta> = { args: { variant: "framed" } };
