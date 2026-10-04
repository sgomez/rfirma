//! Las historias del selector de certificado: cerrado con y sin elegido, buscando, deshabilitado y abierto con la lista agrupada.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { CertificateSelect } from "./CertificateSelect";
import {
  ENTITY_CERTIFICATE,
  PERSONAL_CERTIFICATE,
  STORY_CERTIFICATES,
} from "./signingStoryFixtures";

const meta = {
  title: "Firma/CertificateSelect",
  component: CertificateSelect,
  parameters: { layout: "centered" },
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

export const Open: Story = {
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("combobox"));
    await expect(within(canvasElement.ownerDocument.body).getByRole("listbox")).toBeVisible();
  },
};
