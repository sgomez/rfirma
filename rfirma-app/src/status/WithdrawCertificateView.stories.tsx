//! Las historias del velo de la retirada del certificado: la pregunta, el avance y cada desenlace, sobre el panel de estado.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { StatusView } from "./StatusView";
import { memoryStatus } from "./status";
import {
  caInstalledEverywhere,
  sitesHandledByRfirma,
  withdrawalDone,
  withdrawalHandlerFailed,
  withdrawalPartial,
  withdrawalStores,
} from "./statusStoryData";
import { inStatusWindow } from "./statusStoryFrame";
import { WithdrawCertificateView } from "./WithdrawCertificateView";

const behindTheVeil = (
  <StatusView
    onClose={fn()}
    statusPort={memoryStatus([sitesHandledByRfirma, caInstalledEverywhere])}
  />
);

const meta = {
  title: "Estado/2 · Retirar certificado",
  component: WithdrawCertificateView,
  decorators: [
    (Story) => (
      <>
        {behindTheVeil}
        <Story />
      </>
    ),
    inStatusWindow,
  ],
  parameters: { layout: "centered" },
  args: {
    stores: withdrawalStores,
    report: null,
    onWithdraw: fn(),
    onClose: fn(),
  },
} satisfies Meta<typeof WithdrawCertificateView>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Question: Story = { args: { moment: "question" } };

export const Working: Story = { args: { moment: "working" } };

export const Done: Story = { args: { moment: "result", report: withdrawalDone } };

export const Partial: Story = { args: { moment: "result", report: withdrawalPartial } };

export const HandlerFailed: Story = {
  args: { moment: "result", report: withdrawalHandlerFailed },
};
