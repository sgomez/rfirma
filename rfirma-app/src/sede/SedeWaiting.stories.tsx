//! La historia de la sede en su momento 1, la espera del canal.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { SedeWindow } from "./SedeWindow";
import { storyErrand } from "./sedeStoryPort";

const meta = {
  title: "Sede/1 · Espera",
  component: SedeWindow,
  args: { errands: storyErrand({ kind: "waiting" }) },
} satisfies Meta<typeof SedeWindow>;

export default meta;

export const Waiting: StoryObj<typeof meta> = {};
