//! Las filas y los informes de ejemplo de las historias del panel de estado y de la retirada.

import type { SignalRow, StoreDetail, WithdrawalReport } from "../status";

const base = { action: null, detail: null, candidates: null, restartFirefoxNotice: false };

export const versionUpToDate: SignalRow = {
  ...base,
  signal: "version",
  value: "0.4.1",
  verdict: "correct",
};

export const versionOutdated: SignalRow = {
  ...base,
  signal: "version",
  value: "0.4.1 → 0.5.0",
  verdict: "attention",
  action: { kind: "link", target: "releases" },
};

export const versionChecking: SignalRow = {
  ...base,
  signal: "version",
  value: "",
  verdict: "checking",
};

export const sitesHandledByRfirma: SignalRow = {
  ...base,
  signal: "siteSignature",
  value: "rFirma",
  verdict: "correct",
};

export const sitesNotConfigured: SignalRow = {
  ...base,
  signal: "siteSignature",
  value: "",
  verdict: "attention",
  action: { kind: "choice", target: "rfirma.desktop" },
};

export const sitesUnavailable: SignalRow = {
  ...base,
  signal: "siteSignature",
  value: "",
  verdict: "notApplicable",
  detail: { kind: "handlerDiagnosis", desktopFile: "me.sgomez.rfirma.desktop" },
};

export const sitesUnavailableOutsideFlatpak: SignalRow = {
  ...base,
  signal: "siteSignature",
  value: "",
  verdict: "notApplicable",
};

export const sitesWithTwoCandidates: SignalRow = {
  ...base,
  signal: "siteSignature",
  value: "AutoFirma",
  verdict: "attention",
  action: { kind: "choice", target: "rfirma.desktop" },
  candidates: [
    { id: "rfirma.desktop", name: "rFirma", selected: false },
    { id: "autofirma.desktop", name: "AutoFirma", selected: true },
  ],
};

export const caChecking: SignalRow = {
  ...base,
  signal: "localCaCertificate",
  value: "",
  verdict: "checking",
};

export const caMissing: SignalRow = {
  ...base,
  signal: "localCaCertificate",
  value: "0/2",
  verdict: "incorrect",
  action: { kind: "repair", target: "install" },
  detail: {
    kind: "trust",
    stores: [
      { brand: "firefox", trusted: false },
      { brand: "chrome", trusted: false },
    ],
  },
};

export const caHalfInstalled: SignalRow = {
  ...caMissing,
  value: "1/2",
  verdict: "attention",
  detail: {
    kind: "trust",
    stores: [
      { brand: "firefox", trusted: true },
      { brand: "chrome", trusted: false },
    ],
  },
};

export const caInstalledEverywhere: SignalRow = {
  ...base,
  signal: "localCaCertificate",
  value: "2/2",
  verdict: "correct",
  detail: {
    kind: "trust",
    stores: [
      { brand: "firefox", trusted: true },
      { brand: "chrome", trusted: true },
    ],
  },
};

export const caInstalledNeedsFirefoxRestart: SignalRow = {
  ...caInstalledEverywhere,
  restartFirefoxNotice: true,
};

export const noUserCertificates: SignalRow = {
  ...base,
  signal: "userCertificates",
  value: "0",
  verdict: "attention",
  action: { kind: "link", target: "certificateIssuance" },
};

export const someUserCertificates: SignalRow = {
  ...base,
  signal: "userCertificates",
  value: "5",
  verdict: "correct",
  detail: {
    kind: "certificates",
    stores: [
      { brand: "windows", certificates: 1 },
      { brand: "firefox", certificates: 2 },
      { brand: "card", certificates: 1 },
      { brand: "installed", certificates: 1 },
    ],
  },
};

export const withdrawalStores: StoreDetail[] = [
  { brand: "firefox", trusted: true },
  { brand: "chrome", trusted: true },
];

export const withdrawalDone: WithdrawalReport = {
  handler: { kind: "withdrawn" },
  stores: [
    { brand: "firefox", outcome: { kind: "withdrawn" } },
    { brand: "chrome", outcome: { kind: "wasNotThere" } },
  ],
};

export const withdrawalPartial: WithdrawalReport = {
  handler: { kind: "withdrawn" },
  stores: [
    { brand: "firefox", outcome: { kind: "withdrawn" } },
    { brand: "chrome", outcome: { kind: "failed", reason: "perfil en uso" } },
  ],
};

export const withdrawalHandlerFailed: WithdrawalReport = {
  handler: { kind: "failed", reason: "no se pudo escribir la asociación" },
  stores: [
    { brand: "firefox", outcome: { kind: "withdrawn" } },
    { brand: "chrome", outcome: { kind: "withdrawn" } },
  ],
};
