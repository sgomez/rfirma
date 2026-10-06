//! Las historias del panel de firma antes de firmar: cada estado del certificado, la firma visible con sus páginas y modelos, las firmas previas, firmando y el error.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import {
  inPanelColumn,
  panelStoryParameters,
  UNWRITABLE_DESTINATION,
  WRITABLE_DESTINATION,
} from "../../.storybook/decorators/signingPanel";
import {
  aReport,
  aSignature,
  aSignatureBy,
  ENTITY_CERTIFICATE,
  EXPIRED_ONLY_REPORT,
  MIXED_REPORT,
  PERSONAL_CERTIFICATE,
  STORY_CERTIFICATES,
  STORY_RUBRIC,
  VALID_CLOSING,
} from "../../.storybook/fixtures/signing";
import { placementStateOf } from "../placement/placementFixtures";
import { SigningPanel } from "./SigningPanel";
import {
  aCertificateSection,
  aDestinationSection,
  aRubricSection,
  aSigningSection,
  aVisibleSignatureSection,
} from "./signingSectionFixtures";
import { DEFAULT_VISIBLE_SIGNATURE } from "./visibleSignature";

const RECT = { x0: 100, y0: 100, x1: 300, y1: 180 };

const visible = { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true };

const entityChosen = {
  kind: "chosen",
  certificate: ENTITY_CERTIFICATE,
  certificates: STORY_CERTIFICATES,
} as const;

const meta = {
  title: "Panel de firma/1 · Antes de firmar",
  component: SigningPanel,
  decorators: [inPanelColumn],
  parameters: panelStoryParameters,
  args: {
    document: { id: "doc-1", name: "contrato.pdf", sizeBytes: 2_400_000 },
    previousSignatures: aReport([]),
    certificate: aCertificateSection({
      kind: "chosen",
      certificate: PERSONAL_CERTIFICATE,
      certificates: [PERSONAL_CERTIFICATE],
    }),
    signature: aVisibleSignatureSection(DEFAULT_VISIBLE_SIGNATURE),
    placementState: placementStateOf({ viewedPage: 3, pageCount: 27 }),
    rubric: aRubricSection(),
    destination: aDestinationSection(WRITABLE_DESTINATION),
    signing: aSigningSection(),
    failure: null,
    onOpenHelp: fn(),
  },
} satisfies Meta<typeof SigningPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

const singlePage = { rect: RECT, sets: { single: 3, these: null }, viewedPage: 3, pageCount: 27 };

const singlePageSeal = {
  signature: aVisibleSignatureSection(visible),
  placementState: placementStateOf(singlePage),
} satisfies Story["args"];

export const Ready: Story = {};

export const Unchosen: Story = {
  args: {
    certificate: aCertificateSection({ kind: "unchosen", certificates: STORY_CERTIFICATES }),
  },
};

export const Searching: Story = { args: { certificate: aCertificateSection({ kind: "loading" }) } };

export const NoCertificates: Story = {
  args: { certificate: aCertificateSection({ kind: "empty" }) },
};

export const SearchFailed: Story = {
  args: {
    certificate: aCertificateSection({
      kind: "failed",
      failure: { situation: "tokenAbsent", detail: "CKR_TOKEN_NOT_PRESENT", attemptsLeft: null },
    }),
  },
};

export const SeveralCertificates: Story = {
  args: { certificate: aCertificateSection(entityChosen) },
};

export const VisibleSignatureOnePage: Story = { args: singlePageSeal };

export const VisibleSignatureSeveralPages: Story = {
  args: {
    signature: aVisibleSignatureSection(visible),
    placementState: placementStateOf({
      ...singlePage,
      sets: { single: 3, these: { only: [1, 6] } },
      mode: "these",
    }),
  },
};

export const VisibleSignatureEveryPage: Story = {
  args: {
    signature: aVisibleSignatureSection(visible),
    placementState: placementStateOf({ ...singlePage, mode: "all" }),
  },
};

export const VisibleSignatureOnAnotherPage: Story = {
  args: { ...singlePageSeal, placementState: placementStateOf({ ...singlePage, viewedPage: 5 }) },
};

export const VisibleSignatureNotPlaced: Story = {
  args: { signature: aVisibleSignatureSection(visible) },
};

export const RangeOutOfDocument: Story = {
  args: {
    signature: aVisibleSignatureSection(visible),
    placementState: placementStateOf({
      ...singlePage,
      sets: { single: 3, these: { only: [10, 40] } },
      mode: "these",
      pageCount: 6,
    }),
  },
};

export const CompleteModelWithRubric: Story = {
  args: {
    ...singlePageSeal,
    signature: aVisibleSignatureSection({ ...visible, withRubric: true }),
    rubric: aRubricSection({ value: STORY_RUBRIC }),
  },
};

export const RubricOnlyModel: Story = {
  args: {
    ...singlePageSeal,
    signature: aVisibleSignatureSection({
      ...visible,
      withRubric: true,
      content: { model: "rubricOnly" },
    }),
    rubric: aRubricSection({ value: STORY_RUBRIC }),
  },
};

export const RubricWithoutImage: Story = {
  args: {
    ...singlePageSeal,
    signature: aVisibleSignatureSection({ ...visible, withRubric: true }),
  },
};

export const RubricFailed: Story = {
  args: {
    ...singlePageSeal,
    rubric: aRubricSection({
      failure: { situation: "notAnAcceptedImage", detail: "formato no admitido" },
    }),
  },
};

export const CustomModel: Story = {
  args: {
    ...singlePageSeal,
    signature: aVisibleSignatureSection({
      ...visible,
      content: {
        model: "custom",
        phrase: [
          { text: "Visto bueno de " },
          { datum: "signer" },
          { text: ", " },
          { datum: "signedAt" },
        ],
      },
    }),
  },
};

export const PreviousSignaturesAllValid: Story = {
  args: { previousSignatures: aReport([VALID_CLOSING, aSignature({})]) },
};

export const PreviousSignaturesExpired: Story = {
  args: { previousSignatures: EXPIRED_ONLY_REPORT },
};

export const PreviousSignaturesWithProblems: Story = {
  args: { previousSignatures: MIXED_REPORT },
};

export const PreviousSignaturesSameCertificate: Story = {
  args: { previousSignatures: aReport([aSignatureBy(PERSONAL_CERTIFICATE)]) },
};

export const PreviousSignaturesOtherCertificate: Story = {
  args: {
    certificate: aCertificateSection(entityChosen),
    previousSignatures: aReport([
      aSignatureBy(ENTITY_CERTIFICATE, { certificateSerialNumber: "renewed" }),
    ]),
  },
};

export const PreviousSignaturesOnlyAFinding: Story = {
  args: {
    previousSignatures: aReport(
      [aSignature({ idNumber: "other" })],
      ["modifiedAfterLastSignature"],
    ),
  },
};

export const PreviousSignaturesMany: Story = {
  args: {
    previousSignatures: aReport(
      Array.from({ length: 6 }, (_, index) =>
        aSignature({ idNumber: "other", certificateSerialNumber: String(index) }),
      ),
    ),
  },
};

export const ClosedDocument: Story = {
  args: { previousSignatures: { ...aReport([VALID_CLOSING]), closed: true } },
};

export const VisibleSignatureWithoutCertificate: Story = {
  args: {
    signature: aVisibleSignatureSection(visible),
    certificate: aCertificateSection({ kind: "unchosen", certificates: STORY_CERTIFICATES }),
  },
};

export const LongDestinationName: Story = {
  args: {
    destination: aDestinationSection({
      folder: "Documentos",
      name: `contrato-de-arrendamiento-${"largo-".repeat(6)}firmado-2.pdf`,
      writable: true,
    }),
  },
};

export const Signing: Story = {
  args: { ...singlePageSeal, signing: aSigningSection({ running: true }) },
};

export const UnwritableDestination: Story = {
  args: { destination: aDestinationSection(UNWRITABLE_DESTINATION) },
};

export const SigningFailed: Story = {
  args: {
    failure: {
      situation: "certificateNotFound",
      detail: "CKR_DEVICE_REMOVED durante C_Sign (fase: firma)",
    },
  },
};

export const KeyringPinMissing: Story = {
  args: {
    failure: { situation: "keyringPinMissing", detail: "no hay PIN del almacén en el llavero" },
    onEmptyStore: fn(),
  },
};
