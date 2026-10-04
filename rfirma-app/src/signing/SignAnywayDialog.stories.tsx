//! Las historias del diálogo «¿Firmar de todos modos?», según los problemas que trae el documento.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { signingProblems } from "./previousSignatures";
import { SignAnywayDialog } from "./SignAnywayDialog";
import {
  EXPIRED_ONLY_REPORT,
  EXTREME_REPORT,
  MIXED_REPORT,
  UNRECOGNIZED_REPORT,
} from "./signingStoryFixtures";

const meta = {
  title: "Diálogos de firma/2 · Firmar de todos modos",
  component: SignAnywayDialog,
  parameters: { layout: "fullscreen" },
  args: {
    problems: signingProblems(MIXED_REPORT),
    locale: "es",
    onConfirm: fn(),
    onCancel: fn(),
  },
} satisfies Meta<typeof SignAnywayDialog>;

export default meta;

export const FindingAndSignatures: StoryObj<typeof meta> = {};

export const OnlyExpired: StoryObj<typeof meta> = {
  args: { problems: signingProblems(EXPIRED_ONLY_REPORT) },
};

export const UnrecognizedFormat: StoryObj<typeof meta> = {
  args: { problems: signingProblems(UNRECOGNIZED_REPORT) },
};

export const EveryReasonAndFinding: StoryObj<typeof meta> = {
  args: { problems: signingProblems(EXTREME_REPORT) },
};
