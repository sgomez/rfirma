//! Las historias de la sede en su momento 1c, marcar el área de la firma visible sobre el PDF.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { sedeAreaStoryMeta } from "../../.storybook/decorators/sedeWindow";
import type { SedeView } from "./SedeView";
import { blankPdf } from "./testing/fixtures/previousSignatures";
import { sedeErrand } from "./testing/fixtures/sedeView";

const meta = { title: "Flujos/Sede/Marcar la firma", ...sedeAreaStoryMeta } satisfies Meta<
  typeof SedeView
>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Marking: Story = { args: { errand: sedeErrand({ kind: "marking", pdf: blankPdf }) } };

export const UnreadableDocument: Story = {
  args: { errand: sedeErrand({ kind: "marking", pdf: null }) },
};
