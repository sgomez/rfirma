//! Las historias de la sede en su momento 2b, la confirmación que exige el validador, con cada mensaje que sabe contar.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { sedeStoryMeta } from "../../.storybook/decorators/sedeWindow";
import type { SedeView } from "./SedeView";
import { sedeErrand } from "./testing/fixtures/sedeView";

const meta = { title: "Flujos/Sede/Confirmar", ...sedeStoryMeta } satisfies Meta<typeof SedeView>;

export default meta;

type Story = StoryObj<typeof meta>;

const confirming = (messageCode: string): Story => ({
  args: { errand: sedeErrand({ kind: "confirming", messageCode }) },
});

export const ShadowAttackSuspect = confirming("pdfShadowAttackSuspect");

export const ModifiedForm = confirming("signingModifiedPdfForm");

export const CertifiedPdf = confirming("signingCertifiedPdf");

export const UnknownMessage = confirming("unknownValidatorMessage");
