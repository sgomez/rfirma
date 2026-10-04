//! Las historias del panel de estado: cada señal en cada veredicto, con el detalle plegado y desplegado.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { StatusView, type StatusViewProps } from "./StatusView";
import { memoryStatus, type SignalRow } from "./status";
import {
  caChecking,
  caHalfInstalled,
  caInstalledEverywhere,
  caInstalledNeedsFirefoxRestart,
  caMissing,
  noUserCertificates,
  sitesHandledByRfirma,
  sitesNotConfigured,
  sitesUnavailable,
  sitesWithTwoCandidates,
  someUserCertificates,
  versionChecking,
  versionOutdated,
  versionUpToDate,
} from "./statusStoryData";
import { inStatusWindow } from "./statusStoryFrame";

const meta = {
  title: "Estado/1 · Panel",
  component: StatusView,
  decorators: [inStatusWindow],
  parameters: { layout: "centered" },
  args: { onClose: fn() },
} satisfies Meta<typeof StatusView>;

export default meta;

type Story = StoryObj<typeof meta>;

const panel = (
  rows: SignalRow[],
  extra: Pick<StatusViewProps, "initiallyExpanded"> = {},
): Story => ({
  args: { statusPort: memoryStatus(rows), ...extra },
});

export const EverythingCorrect = panel([
  versionUpToDate,
  sitesHandledByRfirma,
  caInstalledEverywhere,
  someUserCertificates,
]);

export const SomethingToRepair = panel([
  versionOutdated,
  sitesNotConfigured,
  caMissing,
  noUserCertificates,
]);

export const Checking = panel([versionChecking, sitesHandledByRfirma, caChecking]);

export const VersionUpToDate = panel([versionUpToDate]);

export const VersionOutdated = panel([versionOutdated]);

export const SitesHandledByRfirma = panel([sitesHandledByRfirma]);

export const SitesNotConfigured = panel([sitesNotConfigured]);

export const SitesUnavailable = panel([sitesUnavailable]);

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

export const UserCertificatesDetailExpanded = panel([someUserCertificates], {
  initiallyExpanded: ["userCertificates"],
});
