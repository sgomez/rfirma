//! La historia de la sede en su momento 2, el consentimiento de una firma de PDF.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { NO_PREVIOUS_SIGNATURES } from "../signing/previousSignatures";
import { SedeWindow } from "./SedeWindow";
import { inSedeWindow } from "./sedeStoryFrame";
import { storyErrand } from "./sedeStoryPort";

const meta = {
  title: "Sede/2 · Consentimiento",
  component: SedeWindow,
  decorators: [inSedeWindow],
  args: {
    consentCountdown: false,
    errands: storyErrand({
      kind: "consent",
      document: {
        title: "Solicitud de subvención 2026",
        pages: 27,
        sizeBytes: 2_400_000,
        round: { kind: "sign" },
        previousSignatures: NO_PREVIOUS_SIGNATURES,
      },
      signs: null,
      signing: "pdf",
      items: null,
      certificates: [
        {
          id: "handle-1",
          label: "FNMT",
          holderName: "ADA LOVELACE BYRON",
          stampedSigner: "ADA LOVELACE BYRON",
          givenName: "ADA",
          surname: "LOVELACE BYRON",
          idNumber: "99999999R",
          organizationIdentifier: null,
          entityName: null,
          issuer: "FNMT-RCM",
          certificateSerialNumber: "1234567890",
          stores: ["installed"],
          status: { kind: "valid", notAfter: 4_102_444_800 },
          remembered: false,
        },
      ],
      narrowed: false,
    }),
  },
} satisfies Meta<typeof SedeWindow>;

export default meta;

export const Consent: StoryObj<typeof meta> = {};
