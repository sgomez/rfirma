//! Las historias de la sección de la firma visible: apagada, sin certificado, encendida y con cada alcance de páginas.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { placementStateOf } from "../placement/testing/fixtures";
import { aVisibleSignatureSection } from "./testing/fixtures";
import { PERSONAL_CERTIFICATE, STORY_CERTIFICATES } from "./testing/storyReports";
import { VisibleSignatureFieldset } from "./VisibleSignatureFieldset";
import { DEFAULT_VISIBLE_SIGNATURE } from "./visibleSignature";

const visible = { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true };

const singlePage = {
  rect: { x0: 100, y0: 100, x1: 300, y1: 180 },
  sets: { single: 3, these: null },
  viewedPage: 3,
  pageCount: 27,
};

const meta = {
  title: "Flujos/Firma/VisibleSignatureFieldset",
  component: VisibleSignatureFieldset,
  parameters: { layout: "centered", designSync: { cardMode: "column" } },
  decorators: [
    (Story) => (
      <div style={{ width: 332 }}>
        <Story />
      </div>
    ),
  ],
  args: {
    signature: aVisibleSignatureSection(DEFAULT_VISIBLE_SIGNATURE),
    certificate: {
      kind: "chosen",
      certificate: PERSONAL_CERTIFICATE,
      certificates: [PERSONAL_CERTIFICATE],
    },
    placementState: placementStateOf({ viewedPage: 3, pageCount: 27 }),
    signing: false,
  },
} satisfies Meta<typeof VisibleSignatureFieldset>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Off: Story = {};

export const WithoutCertificate: Story = {
  args: {
    certificate: { kind: "unchosen", certificates: STORY_CERTIFICATES },
  },
};

export const NotPlaced: Story = {
  args: { signature: aVisibleSignatureSection(visible) },
};

export const OnePage: Story = {
  args: {
    signature: aVisibleSignatureSection(visible),
    placementState: placementStateOf(singlePage),
  },
};

export const SeveralPages: Story = {
  args: {
    signature: aVisibleSignatureSection(visible),
    placementState: placementStateOf({
      ...singlePage,
      sets: { single: 3, these: { only: [1, 6] } },
      mode: "these",
    }),
  },
};

export const EveryPage: Story = {
  args: {
    signature: aVisibleSignatureSection(visible),
    placementState: placementStateOf({ ...singlePage, mode: "all" }),
  },
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

export const Signing: Story = {
  args: {
    signature: aVisibleSignatureSection(visible),
    placementState: placementStateOf(singlePage),
    signing: true,
  },
};
