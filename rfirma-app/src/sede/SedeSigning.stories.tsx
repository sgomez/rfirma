//! Las historias de la sede en su momento 3, firmando y devolviendo la firma.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { sedeMomentMeta } from "../../.storybook/decorators/sedeWindow";
import { SedeSigning } from "./SedeSigning";
import { momentStory, sedeErrand, sedeViewActions } from "./testing/fixtures/sedeView";
import { certificate } from "./testing/fixtures/sedeWindow";

const meta = {
  title: "Flujos/Sede/Firmando",
  ...sedeMomentMeta,
  component: SedeSigning,
  args: { origin: "sede.ejemplo.gob.es", onCancel: sedeViewActions.onCancel },
} satisfies Meta<typeof SedeSigning>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Signing: Story = momentStory(
  { certificate: certificate(), phase: "signing" },
  sedeErrand({ kind: "signing", certificate: certificate(), phase: "signing" }),
);

export const Returning: Story = momentStory(
  { certificate: certificate(), phase: "returning" },
  sedeErrand({ kind: "signing", certificate: certificate(), phase: "returning" }),
);
