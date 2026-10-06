//! Las historias de página de la ventana del estado: el puerto en memoria y la vista pura montada de verdad.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { inStatusWindow } from "../../.storybook/decorators/statusWindow";
import { StatusWindow } from "./StatusWindow";
import { memoryStatus } from "./status";
import {
  caInstalledEverywhere,
  caMissing,
  noUserCertificates,
  sitesHandledByRfirma,
  sitesNotConfigured,
  someUserCertificates,
  versionOutdated,
  versionUpToDate,
} from "./testing/fixtures";

const meta = {
  title: "Pantallas/Estado/1 · Panel",
  component: StatusWindow,
  decorators: [inStatusWindow],
  parameters: { layout: "centered" },
  args: { onClose: fn() },
} satisfies Meta<typeof StatusWindow>;

export default meta;

type Story = StoryObj<typeof meta>;

export const EverythingCorrect: Story = {
  args: {
    statusPort: memoryStatus([
      versionUpToDate,
      sitesHandledByRfirma,
      caInstalledEverywhere,
      someUserCertificates,
    ]),
  },
};

export const SomethingToRepair: Story = {
  args: {
    statusPort: memoryStatus([versionOutdated, sitesNotConfigured, caMissing, noUserCertificates]),
  },
};
