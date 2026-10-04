//! Las historias del diálogo de progreso de la firma, una por etapa en curso.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { SigningProgressDialog } from "./SigningProgressDialog";

const meta = {
  title: "Diálogos de firma/4 · Progreso de firma",
  component: SigningProgressDialog,
  parameters: { layout: "fullscreen" },
  args: { stage: "presign" },
} satisfies Meta<typeof SigningProgressDialog>;

export default meta;

export const Presign: StoryObj<typeof meta> = {};

export const Sign: StoryObj<typeof meta> = { args: { stage: "sign" } };

export const Postsign: StoryObj<typeof meta> = { args: { stage: "postsign" } };
