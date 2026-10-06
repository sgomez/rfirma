//! La página de la ventana de sede: composiciones canónicas de la ventana entera con su marco, a sus dos tamaños y con un desenlace.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { sedeAreaWindowMeta, sedeWindowMeta } from "../../.storybook/decorators/sedeWindow";
import { SedeView } from "./SedeView";
import { blankPdf } from "./testing/fixtures/previousSignatures";
import { sedeErrand, sedeViewActions } from "./testing/fixtures/sedeView";
import { consentStage, signedDocument } from "./testing/fixtures/sedeWindow";

const meta = {
  title: "Pantallas/Ventana de sede",
  parameters: sedeWindowMeta.parameters,
  component: SedeView,
  args: { ...sedeViewActions, consentCountdown: false },
} satisfies Meta<typeof SedeView>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Waiting: Story = {
  decorators: sedeWindowMeta.decorators,
  args: { errand: sedeErrand({ kind: "waiting" }, { origin: null }) },
};

export const Consent: Story = {
  decorators: sedeWindowMeta.decorators,
  args: { errand: sedeErrand(consentStage()) },
};

export const MarkingTheArea: Story = {
  decorators: sedeAreaWindowMeta.decorators,
  args: { errand: sedeErrand({ kind: "marking", pdf: blankPdf }) },
};

export const Signed: Story = {
  decorators: sedeWindowMeta.decorators,
  args: {
    errand: sedeErrand({ kind: "outcome", outcome: { kind: "signed", document: signedDocument } }),
  },
};
