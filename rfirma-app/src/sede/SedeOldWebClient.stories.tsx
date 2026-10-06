//! Las historias de la sede con el aviso del cliente web antiguo.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { sedeMomentMeta } from "../../.storybook/decorators/sedeWindow";
import { SedeOldWebClient } from "./SedeOldWebClient";
import { momentStory, sedeErrand, sedeViewActions } from "./testing/fixtures/sedeView";

const meta = {
  title: "Flujos/Sede/Cliente antiguo",
  ...sedeMomentMeta,
  component: SedeOldWebClient,
  args: { onDismiss: sedeViewActions.onDismissWarning },
} satisfies Meta<typeof SedeOldWebClient>;

export default meta;

type Story = StoryObj<typeof meta>;

export const OldWebClient: Story = momentStory({}, sedeErrand({ kind: "oldWebClient" }));
