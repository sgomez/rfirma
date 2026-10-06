//! Las historias del diálogo de progreso de la firma, una por etapa en curso.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { inDialogWindow } from "../../.storybook/decorators/dialogWindow";
import { SigningProgressDialog } from "./SigningProgressDialog";

const meta = {
  title: "Flujos/Firma/SigningProgressDialog",
  component: SigningProgressDialog,
  parameters: { layout: "centered" },
  decorators: [inDialogWindow],
  args: { stage: "presign" },
} satisfies Meta<typeof SigningProgressDialog>;

export default meta;

export const Presign: StoryObj<typeof meta> = {};

export const Sign: StoryObj<typeof meta> = { args: { stage: "sign" } };

export const Postsign: StoryObj<typeof meta> = { args: { stage: "postsign" } };
