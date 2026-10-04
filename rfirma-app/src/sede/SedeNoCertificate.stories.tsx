//! Las historias de la sede sin certificado utilizable: ninguno instalado o todos excluidos, desde una sede o una orden de terminal.

import type { Meta, StoryObj } from "@storybook/react-vite";
import type { SedeView } from "./SedeView";
import { sedeStoryMeta } from "./sedeStoryFrame";
import { sedeErrand } from "./sedeStoryPort";

const meta = { title: "Sede/5 · Sin certificado", ...sedeStoryMeta } satisfies Meta<
  typeof SedeView
>;

export default meta;

type Story = StoryObj<typeof meta>;

const terminalOrder = { documentPath: "/home/ada/contratos/convenio.pdf" };

export const NoneInstalled: Story = {
  args: { errand: sedeErrand({ kind: "noCertificate", reason: "none", owned: 0 }) },
};

export const ExcludedBySite: Story = {
  args: { errand: sedeErrand({ kind: "noCertificate", reason: "excluded", owned: 2 }) },
};

export const InstallFailed: Story = {
  args: {
    errand: sedeErrand({ kind: "noCertificate", reason: "none", owned: 0 }),
    installFailure: {
      situation: "pkcs12Unreadable",
      detail: "no se puede leer el fichero",
      attemptsLeft: null,
    },
  },
};

export const TerminalNoneInstalled: Story = {
  args: {
    errand: sedeErrand(
      { kind: "noCertificate", reason: "none", owned: 0 },
      { origin: null, terminalOrder },
    ),
  },
};

export const TerminalExcludedByFilter: Story = {
  args: {
    errand: sedeErrand(
      { kind: "noCertificate", reason: "excluded", owned: 2 },
      { origin: null, terminalOrder },
    ),
  },
};
