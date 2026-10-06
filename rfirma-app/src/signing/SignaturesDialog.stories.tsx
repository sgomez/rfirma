//! Las historias del diálogo «Ver firmas», según las firmas que trae el documento.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { inDialogWindow } from "../../.storybook/decorators/dialogWindow";
import { SignaturesDialog } from "./SignaturesDialog";
import {
  ALL_VALID_REPORT,
  EXTREME_REPORT,
  MIXED_REPORT,
  UNRECOGNIZED_REPORT,
} from "./testing/storyReports";

const meta = {
  title: "Flujos/Firma/SignaturesDialog",
  component: SignaturesDialog,
  parameters: { layout: "centered", designSync: { cardMode: "single", viewport: "1340x780" } },
  decorators: [inDialogWindow],
  args: { report: MIXED_REPORT, onClose: fn() },
} satisfies Meta<typeof SignaturesDialog>;

export default meta;

export const FindingAndSignatures: StoryObj<typeof meta> = {};

export const AllValid: StoryObj<typeof meta> = { args: { report: ALL_VALID_REPORT } };

export const UnrecognizedFormat: StoryObj<typeof meta> = { args: { report: UNRECOGNIZED_REPORT } };

export const EveryReasonAndFinding: StoryObj<typeof meta> = { args: { report: EXTREME_REPORT } };
