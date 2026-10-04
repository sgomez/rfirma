//! Las historias de la sede en su momento 2, el consentimiento: cada operación, ronda de firma, aviso y origen que cambia la pantalla.

import type { Meta, StoryObj } from "@storybook/react-vite";
import type { SedeView } from "./SedeView";
import { consentStage, previousSignature, previousSignaturesReport } from "./sedeStoryData";
import { sedeStoryMeta } from "./sedeStoryFrame";
import { sedeErrand } from "./sedeStoryPort";
import { certificate, signedDocument } from "./sedeWindowFixtures";

const meta = { title: "Sede/2 · Consentimiento", ...sedeStoryMeta } satisfies Meta<typeof SedeView>;

export default meta;

type Story = StoryObj<typeof meta>;

const documentWith = (overrides: Partial<typeof signedDocument>) => ({
  ...signedDocument,
  ...overrides,
});

const terminalOrder = { documentPath: "/home/ada/contratos/convenio.pdf" };

export const Consent: Story = { args: { errand: sedeErrand(consentStage()) } };

export const Countdown: Story = {
  args: { errand: sedeErrand(consentStage()), consentCountdown: true },
};

export const SeveralCertificates: Story = {
  args: {
    errand: sedeErrand(
      consentStage({
        certificates: [
          certificate({ remembered: true }),
          certificate({
            id: "handle-2",
            label: "DNIe",
            holderName: "GRACE BREWSTER HOPPER",
            stampedSigner: "GRACE BREWSTER HOPPER",
            givenName: "GRACE",
            surname: "BREWSTER HOPPER",
            idNumber: "00056780Q",
            issuer: "AC DNIE 004",
            certificateSerialNumber: "987654321",
          }),
        ],
      }),
    ),
  },
};

export const NarrowedBySite: Story = {
  args: { errand: sedeErrand(consentStage({ narrowed: true })) },
};

export const UntitledDocument: Story = {
  args: { errand: sedeErrand(consentStage({ document: documentWith({ title: null }) })) },
};

export const Cosign: Story = {
  args: {
    errand: sedeErrand(consentStage({ document: documentWith({ round: { kind: "cosign" } }) })),
  },
};

export const CountersignTree: Story = {
  args: {
    errand: sedeErrand(
      consentStage({ document: documentWith({ round: { kind: "counter", target: "tree" } }) }),
    ),
  },
};

export const CountersignLeafs: Story = {
  args: {
    errand: sedeErrand(
      consentStage({ document: documentWith({ round: { kind: "counter", target: "leafs" } }) }),
    ),
  },
};

export const PreviousSignaturesValid: Story = {
  args: {
    errand: sedeErrand(
      consentStage({
        document: documentWith({
          round: { kind: "cosign" },
          previousSignatures: previousSignaturesReport([previousSignature()]),
        }),
      }),
    ),
  },
};

export const PreviousSignaturesWithProblem: Story = {
  args: {
    errand: sedeErrand(
      consentStage({
        document: documentWith({
          round: { kind: "cosign" },
          previousSignatures: previousSignaturesReport(
            [
              previousSignature(),
              previousSignature({
                name: "Grace Brewster Hopper",
                idNumber: "00056780Q",
                certificateSerialNumber: "2",
                validity: "invalid",
                validityReason: { kind: "modifiedAfterSigning" },
              }),
            ],
            { warningCount: 1, tone: "attention" },
          ),
        }),
      }),
    ),
  },
};

export const Batch: Story = {
  args: { errand: sedeErrand(consentStage({ document: null, signs: 3, signing: null })) },
};

export const LocalBatch: Story = {
  args: {
    errand: sedeErrand(
      consentStage({
        document: null,
        signs: 5,
        signing: null,
        items: [
          { id: "001", signing: "pdf", round: { kind: "sign" } },
          { id: "002", signing: "challenge", round: { kind: "cosign" } },
          { id: "003", signing: "xml", round: { kind: "sign" } },
          { id: "004", signing: "invoice", round: { kind: "counter", target: "tree" } },
          { id: "005", signing: "pdf", round: { kind: "counter", target: "leafs" } },
        ],
      }),
    ),
  },
};

export const IdentityData: Story = {
  args: {
    errand: sedeErrand(consentStage({ document: null, signing: null }), {
      operation: "selectcert",
    }),
  },
};

export const WithoutOrigin: Story = {
  args: { errand: sedeErrand(consentStage(), { origin: null }) },
};

export const TerminalOrder: Story = {
  args: {
    errand: sedeErrand(consentStage({ narrowed: true }), { origin: null, terminalOrder }),
  },
};

export const TerminalOrderWithPreviousSignatures: Story = {
  args: {
    errand: sedeErrand(
      consentStage({
        document: documentWith({
          previousSignatures: previousSignaturesReport([previousSignature()]),
        }),
      }),
      { origin: null, terminalOrder },
    ),
  },
};
