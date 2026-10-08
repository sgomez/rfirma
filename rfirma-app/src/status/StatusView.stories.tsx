//! Las historias de la vista del panel de estado: cada señal en cada veredicto, con el detalle plegado y desplegado.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { inStatusWindow } from "../../.storybook/decorators/statusWindow";
import { StatusView, type StatusViewProps } from "./StatusView";
import type { SignalRow } from "./status";
import {
  caChecking,
  caHalfInstalled,
  caInstalledEverywhere,
  caInstalledNeedsFirefoxRestart,
  caMissing,
  cardReaderDetected,
  cardReaderMissing,
  cardReaderNotSupported,
  noUserCertificates,
  sitesHandledByRfirma,
  sitesNotConfigured,
  sitesUnavailable,
  sitesUnavailableOutsideFlatpak,
  sitesWithTwoCandidates,
  someUserCertificates,
  versionChecking,
  versionOutdated,
  versionUpToDate,
} from "./testing/fixtures";

const meta = {
  title: "Flujos/Estado/StatusView",
  component: StatusView,
  decorators: [inStatusWindow],
  parameters: {
    layout: "centered",
    designSync: { cardMode: "single", primaryStory: "EverythingCorrect", viewport: "1240x760" },
  },
  args: {
    onClose: fn(),
    onRecheck: fn(),
    onAction: fn(),
    onChooseSiteSignatureHandler: fn(),
    onWithdraw: fn(),
  },
} satisfies Meta<typeof StatusView>;

export default meta;

type Story = StoryObj<typeof meta>;

const panel = (
  rows: SignalRow[],
  extra: Pick<StatusViewProps, "initiallyExpanded"> = {},
): Story => ({
  args: { rows, ...extra },
});

export const EverythingCorrect = panel([
  versionUpToDate,
  sitesHandledByRfirma,
  caInstalledEverywhere,
  someUserCertificates,
  cardReaderDetected,
]);

export const SomethingToRepair = panel([
  versionOutdated,
  sitesNotConfigured,
  caMissing,
  noUserCertificates,
  cardReaderMissing,
]);

export const Checking = panel([versionChecking, sitesHandledByRfirma, caChecking]);

export const VersionUpToDate = panel([versionUpToDate]);

export const VersionOutdated = panel([versionOutdated]);

export const SitesHandledByRfirma = panel([sitesHandledByRfirma]);

export const SitesNotConfigured = panel([sitesNotConfigured]);

export const SitesUnavailable = panel([sitesUnavailable]);

export const SitesUnavailableDiagnosisExpanded = panel([sitesUnavailable], {
  initiallyExpanded: ["siteSignature"],
});

export const SitesUnavailableOutsideFlatpak = panel([sitesUnavailableOutsideFlatpak], {
  initiallyExpanded: ["siteSignature"],
});

export const SitesWithTwoCandidates = panel([sitesWithTwoCandidates]);

export const CertificateMissing = panel([caMissing]);

export const CertificateHalfInstalled = panel([caHalfInstalled]);

export const CertificateInstalled = panel([caInstalledEverywhere]);

export const CertificateInstalledNeedsFirefoxRestart = panel([caInstalledNeedsFirefoxRestart]);

export const CertificateDetailExpanded = panel([caHalfInstalled], {
  initiallyExpanded: ["localCaCertificate"],
});

export const NoUserCertificates = panel([noUserCertificates]);

export const SomeUserCertificates = panel([someUserCertificates]);

export const CardReaderDetected = panel([cardReaderDetected]);

export const CardReaderMissing = panel([cardReaderMissing]);

export const CardReaderNotSupported = panel([cardReaderNotSupported]);

export const UserCertificatesDetailExpanded = panel([someUserCertificates], {
  initiallyExpanded: ["userCertificates"],
});
