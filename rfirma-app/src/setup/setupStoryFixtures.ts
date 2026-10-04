//! Las filas de estado del equipo que ven el asistente del primer arranque, sus historias y sus pruebas.

import type { SignalRow } from "../status/status";

export const aVersionRow: SignalRow = {
  signal: "version",
  value: "0.4.1",
  verdict: "correct",
  action: null,
  detail: null,
  candidates: null,
  restartFirefoxNotice: false,
};

export const certificateNotInstalled: SignalRow = {
  signal: "localCaCertificate",
  value: "",
  verdict: "attention",
  action: { kind: "repair", target: "" },
  detail: null,
  candidates: null,
  restartFirefoxNotice: false,
};

export const certificateInstalled: SignalRow = {
  ...certificateNotInstalled,
  verdict: "correct",
  action: null,
};

export const handlerNotOurs: SignalRow = {
  signal: "siteSignature",
  value: "AutoFirma",
  verdict: "attention",
  action: { kind: "choice", target: "rfirma.desktop" },
  detail: null,
  candidates: [
    { id: "autofirma.desktop", name: "AutoFirma", selected: true },
    { id: "rfirma.desktop", name: "rFirma", selected: false },
  ],
  restartFirefoxNotice: false,
};

export const handlerOurs: SignalRow = {
  ...handlerNotOurs,
  value: "rFirma",
  verdict: "correct",
  action: null,
};

export const handlerWithoutAutoFirma: SignalRow = {
  ...handlerNotOurs,
  value: "",
  candidates: [{ id: "rfirma.desktop", name: "rFirma", selected: false }],
};
