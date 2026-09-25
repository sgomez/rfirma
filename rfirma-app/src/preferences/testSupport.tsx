import { screen } from "@testing-library/react";
import type { UserEvent } from "@testing-library/user-event";
import type { Certificate } from "../signing/certificate";
import { renderWithCatalog } from "../testing/render";
import { PreferencesView } from "./PreferencesView";
import type { Preferences } from "./preferences";

export const defaults: Preferences = {
  theme: "system",
  destination: "Documentos",
  offersOriginalFolder: false,
  rememberVisibleSignature: true,
  rememberActivity: true,
  notifyNewVersion: true,
  setupWizardSeen: false,
  consentCountdown: true,
  honourAutomaticSelection: false,
};

/** `2030-01-15T00:00:00Z`, en segundos desde la época. */
const IN_2030 = 1_894_752_000;

/** `2020-01-15T00:00:00Z`, en segundos desde la época. */
export const IN_2020 = 1_579_046_400;

export function anInstalledCertificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "0123456789abcdef",
    label: "FNMT-GEMELO",
    holderName: "Ada Lovelace Byron",
    idNumber: "IDCES-00000000T",
    issuer: "FNMT-RCM",
    store: "installed",
    status: { kind: "valid", notAfter: IN_2030 },
    remembered: false,
    ...overrides,
  };
}

const noop = async () => {};

export function renderView(props: Partial<Parameters<typeof PreferencesView>[0]> = {}) {
  return renderWithCatalog(
    <PreferencesView
      preferences={defaults}
      onChooseDestination={noop}
      onChange={noop}
      onForgetActivity={noop}
      installedCertificates={[]}
      onInstallCertificate={async () => true}
      onRemoveCertificate={noop}
      onClose={noop}
      {...props}
    />,
  );
}

/** Pasa al panel de la pestaña dada, por su nombre en el índice. */
export async function openTab(user: UserEvent, name: string) {
  await user.click(screen.getByRole("tab", { name }));
}
