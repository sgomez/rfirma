//! Las historias del panel de firmas, por momento: el acuse tras firmar y la lectura de firmas con `verify --gui`, con firmas, contrafirmas, sin firmas, formato desconocido y fallos.

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
import type { DocumentFinding, PreviousSignature, SignatureFormat } from "./previousSignatures";
import { type SignaturesMoment, SignaturesPanel } from "./SignaturesPanel";
import type { ReadingState } from "./useSignatureReading";

const OWN_SIGNATURE = aSignature({ name: "LOVELACE BYRON ADA", idNumber: "00000000T" });

const SIGNED_AT = new Date(2026, 9, 3, 11, 4);

const meta = {
  title: "Panel de firma/2 · Firmado",
  component: SignaturesPanel,
  decorators: [inPanelColumn],
  parameters: panelStoryParameters,
  args: {
    documentName: "contrato.pdf",
    destination: WRITABLE_DESTINATION,
    onOpenDocument: fn(),
    onOpenFolder: fn(),
    failure: null,
    ...panelActions,
  },
} satisfies Meta<typeof SignaturesPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

function acknowledgement(findings: readonly DocumentFinding[] = []): SignaturesMoment {
  return {
    kind: "acknowledgement",
    signedAt: SIGNED_AT,
    signatures: [VALID_CLOSING, OWN_SIGNATURE],
    findings,
    onChangeDestination: fn(),
  };
}

function reading(state: ReadingState, signable = true): SignaturesMoment {
  return { kind: "reading", state, signable };
}

function read(
  signatures: readonly PreviousSignature[],
  {
    findings = [],
    format = "pades",
  }: { findings?: readonly DocumentFinding[]; format?: SignatureFormat } = {},
): ReadingState {
  return { kind: "read", signatures, findings, format };
}

export const JustSigned: Story = { args: { moment: acknowledgement() } };

export const JustSignedAfterChangedDocument: Story = {
  args: { moment: acknowledgement(["modifiedAfterLastSignature"]) },
};

export const VerifyWithSignatures: Story = {
  args: { moment: reading(read([VALID_CLOSING, OWN_SIGNATURE])) },
};

export const VerifyWithProblems: Story = {
  args: {
    moment: reading(
      read([VALID_CLOSING, EXPIRED, NOT_ADMITTED], { findings: ["formFilledAfterSigning"] }),
    ),
  },
};

export const VerifyCadesWithCountersignatures: Story = {
  args: {
    documentName: "contrato.csig",
    moment: reading(
      read(
        [
          aSignature({
            countersignatures: [aSignature({ name: "GRACE HOPPER", idNumber: "44444444A" })],
          }),
        ],
        { format: "cades" },
      ),
    ),
  },
};

export const VerifyCadesWithNestedCountersignatures: Story = {
  args: {
    documentName: "contrato.csig",
    moment: reading(
      read(
        [
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
        { format: "cades" },
      ),
    ),
  },
};

export const VerifyXades: Story = {
  args: {
    documentName: "factura.xsig",
    moment: reading(read([OWN_SIGNATURE], { format: "xades" })),
  },
};

export const VerifyNotSignable: Story = {
  args: {
    documentName: "contrato.csig",
    moment: reading(read([VALID_CLOSING, OWN_SIGNATURE], { format: "cades" }), false),
  },
};

export const VerifyWithoutSignatures: Story = { args: { moment: reading(read([])) } };

export const VerifyUnrecognizedFormat: Story = {
  args: {
    documentName: "notas.txt",
    moment: reading(read([], { format: "unrecognized" })),
  },
};

export const VerifyReading: Story = { args: { moment: reading({ kind: "reading" }) } };

export const VerifyReadFailed: Story = {
  args: {
    moment: reading({
      kind: "failed",
      failure: { situation: "bridgeFailed", detail: "bridge error: code 7", attemptsLeft: null },
    }),
  },
};

export const OpenFailed: Story = {
  args: {
    moment: acknowledgement(),
    failure: {
      situation: "folderMissing",
      detail: "portal OpenURI: no se pudo abrir",
      attemptsLeft: null,
    },
  },
};
