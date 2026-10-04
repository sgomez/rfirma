//! Las historias del asistente del primer arranque: la bienvenida y la configuración con cada estado de sus dos pasos.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { inMemoryPreferences } from "../preferences/preferences";
import { defaults } from "../preferences/preferencesFixtures";
import { memoryStatus } from "../status/status";
import { SetupWizard } from "./SetupWizard";
import {
  aVersionRow,
  certificateInstalled,
  certificateNotInstalled,
  handlerNotOurs,
  handlerOurs,
  handlerWithoutAutoFirma,
} from "./setupStoryFixtures";

const meta = {
  title: "Primer arranque",
  component: SetupWizard,
  parameters: { layout: "centered" },
  decorators: [
    (Story) => (
      <div
        style={{
          width: 1100,
          height: 640,
          position: "relative",
          transform: "translateZ(0)",
          overflow: "hidden",
          border: "1px solid var(--rf-border-subtle)",
          borderRadius: "var(--rf-radius-lg)",
          boxShadow: "var(--rf-shadow-elevated)",
        }}
      >
        <Story />
      </div>
    ),
  ],
  args: {
    seen: false,
    preferences: inMemoryPreferences(defaults),
    statusPort: memoryStatus([aVersionRow, certificateNotInstalled, handlerNotOurs]),
    menuAnchor: "header",
    onFinish: fn(),
    onOpenStatus: fn(),
    onOpenPreferences: fn(),
    onOpenHelp: fn(),
    onOpenAbout: fn(),
  },
} satisfies Meta<typeof SetupWizard>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Welcome: Story = { name: "1 · Bienvenida" };

export const NothingDone: Story = {
  name: "2 · Configuración, nada hecho",
  args: { initialStep: 2 },
};

export const BothDone: Story = {
  name: "2 · Configuración, todo hecho",
  args: {
    initialStep: 2,
    statusPort: memoryStatus([aVersionRow, certificateInstalled, handlerOurs]),
  },
};

export const CertificateDoneHandlerPending: Story = {
  name: "2 · Configuración, certificado instalado",
  args: {
    initialStep: 2,
    statusPort: memoryStatus([aVersionRow, certificateInstalled, handlerNotOurs]),
  },
};

export const WithoutAutoFirma: Story = {
  name: "2 · Configuración, sin AutoFirma",
  args: {
    initialStep: 2,
    statusPort: memoryStatus([aVersionRow, certificateNotInstalled, handlerWithoutAutoFirma]),
  },
};

export const ProtectionOff: Story = {
  name: "2 · Configuración, sin la espera de la sede",
  args: {
    initialStep: 2,
    preferences: inMemoryPreferences({ ...defaults, consentCountdown: false }),
  },
};
