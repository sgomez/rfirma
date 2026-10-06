//! Las historias de la sede en su momento 2, el consentimiento: cada operación, ronda de firma, aviso y origen que cambia la pantalla.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { sedeMomentMeta } from "../../.storybook/decorators/sedeWindow";
import type { ErrandStage } from "./errand";
import { SedeConsent } from "./SedeConsent";
import { previousSignature, previousSignaturesReport } from "./testing/fixtures/previousSignatures";
import { momentProps, sedeErrand, sedeViewActions } from "./testing/fixtures/sedeView";
import { certificate, consentStage, signedDocument } from "./testing/fixtures/sedeWindow";

const meta = {
  title: "Flujos/Sede/Consentimiento",
  ...sedeMomentMeta,
  component: SedeConsent,
  args: {
    countdown: false,
    onConsent: sedeViewActions.onConsent,
    onCancel: sedeViewActions.onCancel,
  },
  parameters: {
    ...sedeMomentMeta.parameters,
    designSync: { cardMode: "single", primaryStory: "Consent" },
  },
} satisfies Meta<typeof SedeConsent>;

export default meta;

type Story = StoryObj<typeof meta>;

const consentProps = (
  stage: Extract<ErrandStage, { kind: "consent" }>,
  errand: Parameters<typeof sedeErrand>[1] = {},
) => ({ ...momentProps(sedeErrand(stage, errand)), stage });

const documentWith = (overrides: Partial<typeof signedDocument>) => ({
  ...signedDocument,
  ...overrides,
});

const terminalOrder = { documentPath: "/home/ada/contratos/convenio.pdf" };

export const Consent: Story = { args: { ...consentProps(consentStage()) } };

export const Countdown: Story = {
  args: { ...consentProps(consentStage()), countdown: true },
};

export const SeveralCertificates: Story = {
  args: {
    ...consentProps(
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

export const SignChallenge: Story = {
  args: { ...consentProps(consentStage({ document: null, signing: "challenge" })) },
};

export const SignXml: Story = {
  args: { ...consentProps(consentStage({ document: null, signing: "xml" })) },
};

export const SignInvoice: Story = {
  args: { ...consentProps(consentStage({ document: null, signing: "invoice" })) },
};

export const NarrowedBySite: Story = {
  args: { ...consentProps(consentStage({ narrowed: true })) },
};

export const Sha1Allowed: Story = {
  args: { ...consentProps(consentStage({ sha1Allowed: true })) },
};

export const BatchAsksForSha1: Story = {
  args: {
    ...consentProps(consentStage({ document: null, signing: null, signs: 3, sha1ToAllow: true })),
  },
};

export const SingleSignAsksForSha1: Story = {
  args: { ...consentProps(consentStage({ sha1ToAllow: true })) },
};

export const LocalBatchAsksForSha1: Story = {
  args: {
    ...consentProps(
      consentStage({
        document: null,
        signs: 3,
        signing: null,
        sha1ToAllow: true,
        items: [
          { id: "001", signing: "pdf", round: { kind: "sign" } },
          { id: "002", signing: "challenge", round: { kind: "cosign" } },
          { id: "003", signing: "xml", round: { kind: "sign" } },
        ],
      }),
    ),
  },
};

export const UntitledDocument: Story = {
  args: { ...consentProps(consentStage({ document: documentWith({ title: null }) })) },
};

export const Cosign: Story = {
  args: {
    ...consentProps(consentStage({ document: documentWith({ round: { kind: "cosign" } }) })),
  },
};

export const CountersignTree: Story = {
  args: {
    ...consentProps(
      consentStage({ document: documentWith({ round: { kind: "counter", target: "tree" } }) }),
    ),
  },
};

export const CountersignLeafs: Story = {
  args: {
    ...consentProps(
      consentStage({ document: documentWith({ round: { kind: "counter", target: "leafs" } }) }),
    ),
  },
};

export const PreviousSignaturesValid: Story = {
  args: {
    ...consentProps(
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
    ...consentProps(
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

export const PreviousSignatureBySameCertificate: Story = {
  args: {
    ...consentProps(
      consentStage({
        document: documentWith({
          previousSignatures: previousSignaturesReport([
            previousSignature({
              issuer: certificate().issuer,
              certificateSerialNumber: certificate().certificateSerialNumber,
            }),
          ]),
        }),
      }),
    ),
  },
};

export const Batch: Story = {
  args: { ...consentProps(consentStage({ document: null, signs: 3, signing: null })) },
};

export const LocalBatch: Story = {
  args: {
    ...consentProps(
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
    ...consentProps(consentStage({ document: null, signing: null }), {
      operation: "selectcert",
    }),
  },
};

export const IdentityDataWithoutOrigin: Story = {
  args: {
    ...consentProps(consentStage({ document: null, signing: null }), {
      operation: "selectcert",
      origin: null,
    }),
  },
};

export const WithoutOrigin: Story = {
  args: { ...consentProps(consentStage(), { origin: null }) },
};

export const TerminalOrder: Story = {
  args: {
    ...consentProps(consentStage({ narrowed: true }), { origin: null, terminalOrder }),
  },
};

export const TerminalOrderWithPreviousSignatures: Story = {
  args: {
    ...consentProps(
      consentStage({
        document: documentWith({
          previousSignatures: previousSignaturesReport([previousSignature()]),
        }),
      }),
      { origin: null, terminalOrder },
    ),
  },
};
