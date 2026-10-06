//! Las historias del velo de la retirada del certificado: la pregunta, el avance y cada desenlace, sobre el panel de estado.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { inStatusWindow } from "../../.storybook/decorators/statusWindow";
import { StatusView } from "./StatusView";
import {
  caInstalledEverywhere,
  sitesHandledByRfirma,
  withdrawalDone,
  withdrawalHandlerFailed,
  withdrawalPartial,
  withdrawalStores,
} from "./testing/fixtures";
import { WithdrawCertificateView } from "./WithdrawCertificateView";

const behindTheVeil = (
  <StatusView
    rows={[sitesHandledByRfirma, caInstalledEverywhere]}
    onClose={fn()}
    onRecheck={fn()}
    onAction={fn()}
    onChooseSiteSignatureHandler={fn()}
    onWithdraw={fn()}
  />
);

const meta = {
  title: "Flujos/Estado/WithdrawCertificateView",
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
  parameters: {
    layout: "centered",
    designSync: { cardMode: "single", primaryStory: "Question", viewport: "1240x760" },
  },
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
