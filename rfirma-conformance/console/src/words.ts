import type { Assistance } from "./contract/Assistance";
import type { CheckView } from "./contract/CheckView";
import type { ClientKind } from "./contract/ClientKind";
import type { InMaster } from "./contract/InMaster";
import type { KnownBug } from "./contract/KnownBug";
import type { ResultName } from "./contract/ResultName";
import type { Summary } from "./contract/Summary";

export const RESULTS: readonly ResultName[] = [
  "CONFORME",
  "NO CONFORME",
  "NO OBSERVABLE",
  "PENDIENTE",
];

export const resultTone: Record<ResultName, string> = {
  CONFORME: "ok",
  "NO CONFORME": "fail",
  "NO OBSERVABLE": "unobservable",
  PENDIENTE: "pending",
};

export function countOf(summary: Summary, result: ResultName): number {
  switch (result) {
    case "CONFORME":
      return summary.compliant;
    case "NO CONFORME":
      return summary.noncompliant;
    case "NO OBSERVABLE":
      return summary.not_observable;
    case "PENDIENTE":
      return summary.pending;
  }
}

export function rowsWith(summary: Summary, result: ResultName): number {
  return countOf(summary, result) + (result === "NO CONFORME" ? summary.deprecated : 0);
}

export const DEPRECATED_LABEL = "Formato deprecado";

export const DEPRECATED_REASON =
  "El manual de AutoFirma lo desaconseja (MCF, §8): AutoFirma lo soporta y rFirma no, y su NO CONFORME no cuenta como fallo.";

export const assistanceName: Record<Assistance, string> = {
  none: "nada: es automática",
  click: "elegir o pulsar en un diálogo",
  person: "fijarte en lo que pasa y contestar",
};

const inMasterName: Record<InMaster, string> = {
  present: "sigue en master",
  fixed: "corregido en master",
  partial: "corregido a medias en master",
};

export function bugLabel(bug: KnownBug): string {
  return `Bug AutoFirma 1.9.2 · ${inMasterName[bug.master]}`;
}

export function isAnExpectedFailure(check: CheckView, kind: ClientKind): boolean {
  return kind === "autofirma" && check.bug !== null && check.state === "NO CONFORME";
}

export function clientName(kind: ClientKind | null | undefined): string {
  if (kind === "autofirma") return "AutoFirma";
  if (kind === "rfirma") return "rFirma";
  return "cliente desconocido";
}

const seconds = new Intl.NumberFormat("es-ES", {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1,
});

export function duration(ms: number): string {
  return ms < 60_000 ? `${seconds.format(ms / 1000)} s` : clock(ms);
}

export function clock(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(total / 60);
  const rest = String(total % 60).padStart(2, "0");
  return `${minutes}:${rest}`;
}

const day = new Intl.DateTimeFormat("es-ES", { day: "numeric", month: "short", year: "numeric" });

export function calendarDate(text: string | null | undefined): string {
  if (!text) return "";
  const parsed = new Date(text.length === 10 ? `${text}T12:00:00` : text);
  return Number.isNaN(parsed.getTime()) ? text : day.format(parsed);
}
