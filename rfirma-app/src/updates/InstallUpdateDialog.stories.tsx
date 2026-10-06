//! La historia de la confirmación de instalar la versión nueva, que es como se abre el diálogo.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { InstallUpdateDialog } from "./InstallUpdateDialog";
import { inMemoryVersionCheck } from "./newVersion";

const newVersion = { version: "0.4.1", installable: true };

const meta = {
  title: "Pantallas/Acerca de/2 · Instalar actualización",
  component: InstallUpdateDialog,
  parameters: { layout: "fullscreen" },
  args: { newVersion, versions: inMemoryVersionCheck(newVersion), onClose: fn() },
} satisfies Meta<typeof InstallUpdateDialog>;

export default meta;

export const Confirming: StoryObj<typeof meta> = {};
