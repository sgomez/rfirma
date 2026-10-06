//! Las historias de la sede en su momento 1c, marcar el área de la firma visible sobre el PDF.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { sedeAreaMomentMeta } from "../../.storybook/decorators/sedeWindow";
import { SedeMarking } from "./SedeMarking";
import { blankPdf } from "./testing/fixtures/previousSignatures";
import { momentStory, sedeErrand, sedeViewActions } from "./testing/fixtures/sedeView";

const meta = {
  title: "Flujos/Sede/Marcar la firma",
  ...sedeAreaMomentMeta,
  component: SedeMarking,
  args: { onMark: sedeViewActions.onMarkArea, onCancel: sedeViewActions.onCancel },
} satisfies Meta<typeof SedeMarking>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Marking: Story = momentStory(
  { pdf: blankPdf },
  sedeErrand({ kind: "marking", pdf: blankPdf }),
);

export const UnreadableDocument: Story = momentStory(
  { pdf: null },
  sedeErrand({ kind: "marking", pdf: null }),
);
