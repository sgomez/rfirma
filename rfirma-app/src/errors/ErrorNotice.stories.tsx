//! Las historias del aviso de error: un fallo con reintento, el de firma con el documento intacto y los que piden otra acción.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { ErrorNotice } from "./ErrorNotice";

const meta = {
  title: "Dominio/Errores/ErrorNotice",
  component: ErrorNotice,
  parameters: { layout: "centered", designSync: { cardMode: "column" } },
  decorators: [
    (Story) => (
      <div style={{ width: 380 }}>
        <Story />
      </div>
    ),
  ],
  args: { situation: "bridgeFailed", technicalDetail: "bridge error: code 7", onOpenHelp: fn() },
} satisfies Meta<typeof ErrorNotice>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Retry: Story = {};

export const SigningFailed: Story = { args: { documentUnchanged: true } };

export const SigningFailedWithoutRetry: Story = {
  args: { situation: "certificateExpired", documentUnchanged: true },
};

export const CheckToken: Story = { args: { situation: "tokenAbsent" } };

export const WithoutDetail: Story = { args: { situation: "keyKindUnsupported" } };

export const KeyringPinMissing: Story = {
  args: { situation: "keyringPinMissing", onEmptyStore: fn() },
};

export const Reload: Story = { args: { situation: "renderFailed", onReload: fn() } };
