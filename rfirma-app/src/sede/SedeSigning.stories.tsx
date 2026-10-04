//! Las historias de la sede en su momento 3, firmando y devolviendo la firma, y las del fichero que pide el diálogo del portal.

import type { Meta, StoryObj } from "@storybook/react-vite";
import type { SedeView } from "./SedeView";
import { sedeStoryMeta } from "./sedeStoryFrame";
import { sedeErrand } from "./sedeStoryPort";
import { certificate } from "./sedeWindowFixtures";

const meta = { title: "Sede/3 · Firmando", ...sedeStoryMeta } satisfies Meta<typeof SedeView>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Signing: Story = {
  args: {
    errand: sedeErrand({ kind: "signing", certificate: certificate(), phase: "signing" }),
  },
};

export const Returning: Story = {
  args: {
    errand: sedeErrand({ kind: "signing", certificate: certificate(), phase: "returning" }),
  },
};

export const SavingNamedFile: Story = {
  args: { errand: sedeErrand({ kind: "saving", filename: "informe.pdf" }) },
};

export const SavingUnnamedFile: Story = {
  args: { errand: sedeErrand({ kind: "saving", filename: null }) },
};

export const SavingUnwritableDestination: Story = {
  args: { errand: sedeErrand({ kind: "saving", filename: "informe.pdf", unwritable: true }) },
};

export const LoadingOneFile: Story = {
  args: { errand: sedeErrand({ kind: "loading", multiple: false }) },
};

export const LoadingSeveralFiles: Story = {
  args: { errand: sedeErrand({ kind: "loading", multiple: true }) },
};
