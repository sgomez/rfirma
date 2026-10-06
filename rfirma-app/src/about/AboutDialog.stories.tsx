//! Las historias de Acerca de: al día, con versión nueva instalable o solo anunciada, y con los avisos apagados.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { inMemoryVersionCheck, type NewVersion } from "../updates/newVersion";
import { AboutDialog } from "./AboutDialog";

const installable: NewVersion = { version: "0.4.1", installable: true };
const announcedOnly: NewVersion = { version: "0.4.1", installable: false };

const meta = {
  title: "Pantallas/Acerca de/1 · Diálogo",
  component: AboutDialog,
  parameters: { layout: "fullscreen" },
  args: {
    version: "0.4.0",
    newVersion: null,
    versions: inMemoryVersionCheck(null),
    offerUpdate: true,
    onOpenSourceCode: fn(),
    onClose: fn(),
  },
} satisfies Meta<typeof AboutDialog>;

export default meta;

type Story = StoryObj<typeof meta>;

export const UpToDate: Story = {};

export const NewVersionInstallable: Story = {
  args: { newVersion: installable, versions: inMemoryVersionCheck(installable) },
};

export const NewVersionAnnouncedOnly: Story = {
  args: { newVersion: announcedOnly, versions: inMemoryVersionCheck(announcedOnly) },
};

export const NewVersionWithoutOffer: Story = {
  args: {
    newVersion: installable,
    versions: inMemoryVersionCheck(installable),
    offerUpdate: false,
  },
};
