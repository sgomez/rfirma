//! Las historias de la sede en su momento 2b, la confirmación que exige el validador, con cada mensaje que sabe contar.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { sedeMomentMeta } from "../../.storybook/decorators/sedeWindow";
import { SedeConfirm } from "./SedeConfirm";
import { momentStory, sedeErrand, sedeViewActions } from "./testing/fixtures/sedeView";

const meta = {
  title: "Flujos/Sede/Confirmar",
  ...sedeMomentMeta,
  parameters: { ...sedeMomentMeta.parameters, designSync: { cardMode: "column" } },
  component: SedeConfirm,
  args: { onConfirm: sedeViewActions.onConfirmSignatures, onCancel: sedeViewActions.onCancel },
} satisfies Meta<typeof SedeConfirm>;

export default meta;

type Story = StoryObj<typeof meta>;

const confirming = (messageCode: string): Story =>
  momentStory({ messageCode }, sedeErrand({ kind: "confirming", messageCode }));

export const ShadowAttackSuspect = confirming("pdfShadowAttackSuspect");

export const ModifiedForm = confirming("signingModifiedPdfForm");

export const CertifiedPdf = confirming("signingCertifiedPdf");

export const UnknownMessage = confirming("unknownValidatorMessage");
