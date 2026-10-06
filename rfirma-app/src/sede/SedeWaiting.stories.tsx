//! Las historias de la sede en la espera del canal, con sus dos reparaciones.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { sedeMomentMeta } from "../../.storybook/decorators/sedeWindow";
import { SedeWaiting } from "./SedeWaiting";
import { momentStory, sedeErrand, sedeViewActions } from "./testing/fixtures/sedeView";

const meta = {
  title: "Flujos/Sede/Espera",
  ...sedeMomentMeta,
  parameters: { ...sedeMomentMeta.parameters, designSync: { cardMode: "column" } },
  component: SedeWaiting,
  args: { onInstallLocalCa: sedeViewActions.onInstallLocalCa, onCancel: sedeViewActions.onCancel },
} satisfies Meta<typeof SedeWaiting>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Waiting: Story = momentStory(
  { moment: "connecting" },
  sedeErrand({ kind: "waiting" }, { origin: null }),
);

export const Unreachable: Story = momentStory(
  { moment: "unreachable" },
  sedeErrand({ kind: "unreachable" }),
);

export const NoChannel: Story = momentStory(
  { moment: "unreachable" },
  sedeErrand({ kind: "noChannel", reason: "channelNotOpened" }),
);
