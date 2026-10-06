//! Las historias del visor de documento: el estado vacío, el documento, la firma visible y los fallos.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { RecentsSection } from "../documents/RecentRows";
import { storyRecents } from "../documents/testing/fixtures";
import type { Placement } from "../placement/pageSets";
import { DocumentViewer } from "./DocumentViewer";
import { storyPdf } from "./testing/storyPdf";

const meta = {
  title: "Flujos/Documentos/DocumentViewer",
  component: DocumentViewer,
  parameters: { layout: "centered", designSync: { cardMode: "column" } },
  decorators: [
    (Story) => (
      <div style={{ width: 800, height: 560, display: "grid" }}>
        <Story />
      </div>
    ),
  ],
  args: { pdf: null, placement: null, onMove: fn(), onTrace: fn(), onOpen: fn() },
} satisfies Meta<typeof DocumentViewer>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Empty: Story = {};

export const EmptyWithRecents: Story = {
  args: {
    emptyExtra: <RecentsSection recents={storyRecents} onSelect={fn()} onClear={fn()} />,
  },
};

export const WithDocument: Story = { args: { pdf: storyPdf() } };

export const WithSignatureBox: Story = {
  args: {
    pdf: storyPdf(),
    placement: { rect: { x0: 50, y0: 60, x1: 250, y1: 140 }, pages: { only: [1] } },
  },
};

export const SignatureBoxOnAllPages: Story = {
  args: {
    pdf: storyPdf(),
    placement: { rect: { x0: 50, y0: 60, x1: 250, y1: 140 }, pages: "all" },
  },
};

export const WithoutPreview: Story = { args: { withoutPreview: "Presupuesto.docx" } };

export const FailureNotAPdf: Story = {
  args: { failure: { situation: "notAPdf", detail: "invalid PDF structure" } },
};

export const FailureOverDocument: Story = {
  args: {
    pdf: storyPdf(),
    failure: { situation: "documentEncrypted", detail: "password required" },
  },
};

const sealed: Placement = { rect: { x0: 50, y0: 60, x1: 250, y1: 140 }, pages: { only: [1] } };

export const StampUnplaced: Story = {
  args: { pdf: storyPdf(), placement: null, stamp: { kind: "unplaced" } },
};

export const StampNoCertificate: Story = {
  args: {
    pdf: storyPdf(),
    placement: sealed,
    stamp: { kind: "noCertificate" },
    rubricGap: "beside",
  },
};

export const StampComposed: Story = {
  args: { pdf: storyPdf(), placement: sealed, stamp: { kind: "composed" } },
};

export const StampFrozen: Story = {
  args: { pdf: storyPdf(), placement: sealed, stamp: { kind: "frozen" } },
};

export const StampOnDemand: Story = {
  args: { pdf: storyPdf(), placement: sealed, stamp: { kind: "onDemand" }, onComposeStamp: fn() },
};

export const StampComposing: Story = {
  args: { pdf: storyPdf(), placement: sealed, stamp: { kind: "composing" } },
};

export const StampFailed: Story = {
  args: {
    pdf: storyPdf(),
    placement: sealed,
    stamp: {
      kind: "failed",
      failure: { situation: "documentUnreadable", detail: "password required" },
    },
    onComposeStamp: fn(),
  },
};

export const RubricGapBesideText: Story = {
  args: { pdf: storyPdf(), placement: sealed, stamp: { kind: "composed" }, rubricGap: "beside" },
};

export const RubricGapFillingBox: Story = {
  args: { pdf: storyPdf(), placement: sealed, stamp: { kind: "composing" }, rubricGap: "fill" },
};
