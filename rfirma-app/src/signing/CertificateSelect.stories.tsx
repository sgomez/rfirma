//! Las historias del selector de certificado: cerrado con y sin elegido, buscando, deshabilitado y abierto con la lista agrupada.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { CertificateSelect } from "./CertificateSelect";
import {
  ENTITY_CERTIFICATE,
  PERSONAL_CERTIFICATE,
  STORY_CERTIFICATES,
} from "./testing/storyReports";

const meta = {
  title: "Dominio/Firma/CertificateSelect",
  component: CertificateSelect,
  parameters: { layout: "centered", designSync: { cardMode: "single", primaryStory: "Open" } },
  decorators: [
    (Story) => (
      <div style={{ width: 332, minHeight: 360 }}>
        <Story />
      </div>
    ),
  ],
  args: {
    certificates: STORY_CERTIFICATES,
    chosen: PERSONAL_CERTIFICATE,
    onChoose: fn(),
  },
} satisfies Meta<typeof CertificateSelect>;

export default meta;

type Story = StoryObj<typeof meta>;

export const ChosenPersonal: Story = {};

export const ChosenOnBehalfOfAnEntity: Story = { args: { chosen: ENTITY_CERTIFICATE } };

export const Unchosen: Story = { args: { chosen: null } };

export const Searching: Story = { args: { certificates: [], chosen: null, searching: true } };

export const Disabled: Story = { args: { disabled: true } };

export const Open: Story = { args: { defaultOpen: true } };
