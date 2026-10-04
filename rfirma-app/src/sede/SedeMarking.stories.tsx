//! Las historias de la sede en su momento 1c, marcar el área de la firma visible sobre el PDF.

import type { Meta, StoryObj } from "@storybook/react-vite";
import type { SedeView } from "./SedeView";
import { blankPdf } from "./sedeStoryData";
import { sedeAreaStoryMeta } from "./sedeStoryFrame";
import { sedeErrand } from "./sedeStoryPort";

const meta = { title: "Sede/1c · Marcar la firma", ...sedeAreaStoryMeta } satisfies Meta<
  typeof SedeView
>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Marking: Story = { args: { errand: sedeErrand({ kind: "marking", pdf: blankPdf }) } };

export const UnreadableDocument: Story = {
  args: { errand: sedeErrand({ kind: "marking", pdf: null }) },
};
