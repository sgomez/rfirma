//! Las historias del diálogo de páginas sin firma visible, según cuántas páginas se caen.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { inDialogWindow } from "../../.storybook/decorators/dialogWindow";
import { UnsealedPagesDialog } from "./UnsealedPagesDialog";

const meta = {
  title: "Flujos/Firma/UnsealedPagesDialog",
  component: UnsealedPagesDialog,
  parameters: { layout: "centered" },
  decorators: [inDialogWindow],
  args: { fallen: 3, onConfirm: fn(), onCancel: fn() },
} satisfies Meta<typeof UnsealedPagesDialog>;

export default meta;

export const SeveralPages: StoryObj<typeof meta> = {};

export const OnePage: StoryObj<typeof meta> = { args: { fallen: 1 } };

export const ManyPages: StoryObj<typeof meta> = { args: { fallen: 12 } };
