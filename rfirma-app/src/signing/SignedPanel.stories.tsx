//! Las historias del panel firmado: tras firmar y con `verify --gui`, con firmas, contrafirmas, sin firmas, formato desconocido y fallos.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import {
  inPanelColumn,
  panelActions,
  panelStoryParameters,
  WRITABLE_DESTINATION,
} from "../../.storybook/decorators/signingPanel";
import {
  aSignature,
  EXPIRED,
  NOT_ADMITTED,
  VALID_CLOSING,
} from "../../.storybook/fixtures/signing";
import { SignedPanel } from "./SignedPanel";

const OWN_SIGNATURE = aSignature({ name: "LOVELACE BYRON ADA", idNumber: "00000000T" });

const meta = {
  title: "Panel de firma/2 · Firmado",
  component: SignedPanel,
  decorators: [inPanelColumn],
  parameters: panelStoryParameters,
  args: {
    documentName: "contrato.pdf",
    signatures: [VALID_CLOSING, OWN_SIGNATURE],
    destination: WRITABLE_DESTINATION,
    onOpenDocument: fn(),
    onOpenFolder: fn(),
    ...panelActions,
  },
} satisfies Meta<typeof SignedPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

const verifyOnly = { onChangeDestination: undefined } satisfies Story["args"];

export const JustSigned: Story = { args: { signedAt: new Date(2026, 9, 3, 11, 4) } };

export const JustSignedAfterChangedDocument: Story = {
  args: {
    signedAt: new Date(2026, 9, 3, 11, 4),
    findings: ["modifiedAfterLastSignature"],
  },
};

export const VerifyWithSignatures: Story = { args: verifyOnly };

export const VerifyWithProblems: Story = {
  args: {
    ...verifyOnly,
    signatures: [VALID_CLOSING, EXPIRED, NOT_ADMITTED],
    findings: ["formFilledAfterSigning"],
  },
};

export const VerifyCadesWithCountersignatures: Story = {
  args: {
    ...verifyOnly,
    format: "cades",
    documentName: "contrato.csig",
    signatures: [
      aSignature({
        countersignatures: [aSignature({ name: "GRACE HOPPER", idNumber: "44444444A" })],
      }),
    ],
  },
};

export const VerifyCadesWithNestedCountersignatures: Story = {
  args: {
    ...verifyOnly,
    format: "cades",
    documentName: "contrato.csig",
    signatures: [
      aSignature({
        name: "FIRST SIGNER",
        countersignatures: [
          aSignature({
            name: "CHILD SIGNER",
            countersignatures: [aSignature({ name: "DEEP SIGNER" })],
          }),
          aSignature({ name: "SIBLING SIGNER" }),
        ],
      }),
      aSignature({ name: "SECOND SIGNER" }),
    ],
  },
};

export const VerifyXades: Story = {
  args: {
    ...verifyOnly,
    format: "xades",
    documentName: "factura.xsig",
    signatures: [OWN_SIGNATURE],
  },
};

export const VerifyNotSignable: Story = {
  args: { ...verifyOnly, format: "cades", documentName: "contrato.csig", signable: false },
};

export const VerifyWithoutSignatures: Story = { args: { ...verifyOnly, signatures: [] } };

export const VerifyUnrecognizedFormat: Story = {
  args: { ...verifyOnly, format: "unrecognized", signatures: [], documentName: "notas.txt" },
};

export const VerifyReading: Story = { args: { ...verifyOnly, reading: true, signatures: [] } };

export const VerifyReadFailed: Story = {
  args: {
    ...verifyOnly,
    signatures: [],
    readFailure: {
      situation: "bridgeFailed",
      detail: "bridge error: code 7",
      attemptsLeft: null,
    },
  },
};

export const OpenFailed: Story = {
  args: {
    signedAt: new Date(2026, 9, 3, 11, 4),
    failure: {
      situation: "folderMissing",
      detail: "portal OpenURI: no se pudo abrir",
      attemptsLeft: null,
    },
  },
};
