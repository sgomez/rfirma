//! La historia de `Card`, con control para si está elevada.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { Card } from "./Card";

const meta = {
  title: "Primitivos/Card",
  component: Card,
  args: { children: "Contenido de la tarjeta" },
  argTypes: { elevated: { control: "boolean" } },
} satisfies Meta<typeof Card>;

export default meta;

export const Flat: StoryObj<typeof meta> = {};
export const Elevated: StoryObj<typeof meta> = { args: { elevated: true } };
