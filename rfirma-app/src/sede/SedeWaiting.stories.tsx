//! Las historias de la sede antes de la petición: el aviso del cliente web antiguo y la espera del canal, con sus dos reparaciones.

import type { Meta, StoryObj } from "@storybook/react-vite";
import type { SedeView } from "./SedeView";
import { sedeStoryMeta } from "./sedeStoryFrame";
import { sedeErrand } from "./sedeStoryPort";

const meta = { title: "Sede/1 · Espera", ...sedeStoryMeta } satisfies Meta<typeof SedeView>;

export default meta;

type Story = StoryObj<typeof meta>;

export const OldWebClient: Story = { args: { errand: sedeErrand({ kind: "oldWebClient" }) } };

export const Waiting: Story = {
  args: { errand: sedeErrand({ kind: "waiting" }, { origin: null }) },
};

export const Unreachable: Story = { args: { errand: sedeErrand({ kind: "unreachable" }) } };

export const NoChannel: Story = {
  args: { errand: sedeErrand({ kind: "noChannel", reason: "channelNotOpened" }) },
};
