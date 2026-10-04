//! Las historias de Preferencias: una por sección del índice y por variante de lo que enseña cada una.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { PreferencesView } from "./PreferencesView";
import { anInstalledCertificate, defaults, IN_2020 } from "./preferencesFixtures";

const meta = {
  title: "Preferencias/Pantalla",
  component: PreferencesView,
  parameters: { layout: "centered" },
  decorators: [
    (Story) => (
      <div
        style={{
          width: 900,
          height: 560,
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
    preferences: defaults,
    installedCertificates: [],
    onChooseDestination: fn(async () => {}),
    onChange: fn(async () => {}),
    onForgetActivity: fn(async () => {}),
    onInstallCertificate: fn(async () => true),
    onRemoveCertificate: fn(async () => {}),
    onEmptyStore: fn(async () => {}),
    onClose: fn(),
  },
} satisfies Meta<typeof PreferencesView>;

export default meta;

type Story = StoryObj<typeof meta>;

export const General: Story = { name: "1 · General" };

export const GeneralWithActivityOff: Story = {
  name: "1 · General, sin recordar la actividad",
  args: { preferences: { ...defaults, rememberActivity: false, notifyNewVersion: false } },
};

export const SigningWithoutOriginalFolder: Story = {
  name: "2 · Firma, solo la carpeta",
  args: { initialSection: "signing" },
};

export const SigningNextToTheOriginal: Story = {
  name: "2 · Firma, junto al original",
  args: {
    initialSection: "signing",
    preferences: {
      ...defaults,
      offersOriginalFolder: true,
      destinationMode: "next_to_the_original",
    },
  },
};

export const SigningInTheDestinationFolder: Story = {
  name: "2 · Firma, en la carpeta de destino",
  args: {
    initialSection: "signing",
    preferences: {
      ...defaults,
      offersOriginalFolder: true,
      destinationMode: "in_the_destination_folder",
    },
  },
};

export const NoCertificates: Story = {
  name: "3 · Certificados, ninguno",
  args: { initialSection: "certificates" },
};

export const CertificatesInstalled: Story = {
  name: "3 · Certificados, instalados",
  args: {
    initialSection: "certificates",
    installedCertificates: [
      anInstalledCertificate(),
      anInstalledCertificate({
        id: "2a01",
        entityName: "Analytical Engines S.L.",
        organizationIdentifier: "VATES-B00000000",
      }),
      anInstalledCertificate({
        id: "2a02",
        holderName: "Charles Babbage",
        givenName: "Charles",
        surname: "Babbage",
        status: { kind: "expired", notAfter: IN_2020 },
      }),
    ],
  },
};

export const Appearance: Story = {
  name: "4 · Apariencia",
  args: { initialSection: "appearance" },
};
