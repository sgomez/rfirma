//! La historia de la sede en su momento 4, el rechazo de una petición sin formato.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { SedeWindow } from "./SedeWindow";
import { storyErrand } from "./sedeStoryPort";

const meta = {
  title: "Sede/4 · Rechazo",
  component: SedeWindow,
  args: {
    errands: storyErrand({
      kind: "outcome",
      outcome: { kind: "refused", situation: "missingFormat", detail: "format=" },
    }),
  },
} satisfies Meta<typeof SedeWindow>;

export default meta;

export const Refused: StoryObj<typeof meta> = {};
