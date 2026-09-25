import type { CheckView } from "../contract/CheckView";
import type { LabelView } from "../contract/LabelView";
import type { ReportView } from "../contract/ReportView";
import type { ResultName } from "../contract/ResultName";
import type { SetView } from "../contract/SetView";
import type { Snapshot } from "../contract/Snapshot";
import type { Summary } from "../contract/Summary";

export function aCheck(id: string, state: ResultName = "PENDIENTE"): CheckView {
  return {
    id,
    chapter: "05",
    statement: `Lo que exige ${id}.`,
    citation: "AfirmaWebSocketServerV4.java:57-68",
    warning: null,
    assistance: "none",
    store: "rsa",
    labels: [],
    state,
    observation: state === "PENDIENTE" ? null : "el trámite se completó",
    date: state === "PENDIENTE" ? null : "2026-09-21",
    duration_ms: state === "PENDIENTE" ? null : 3500,
  };
}

export const AN_ADR_LABEL: LabelView = {
  name: "rfirma:adr-0010",
  reason: "Desviación deliberada de rFirma: la decide su ADR-0010",
};

export const A_BUG_LABEL: LabelView = {
  name: "autofirma:bug:1.9.2",
  reason: "BUG-15: Ausencia de validación de cop en signandsave (sigue en master)",
};

export function withLabels(view: ReportView, id: string, labels: LabelView[]): ReportView {
  const sets = view.sets.map((set) =>
    aSet(
      set.name,
      set.checks.map((check) => (check.id === id ? { ...check, labels } : check)),
    ),
  );
  return { ...view, sets, summary: summaryOf(sets.flatMap((set) => set.checks)) };
}

function summaryOf(checks: CheckView[]): Summary {
  const count = (state: ResultName) => checks.filter((check) => check.state === state).length;
  return {
    total: checks.length,
    compliant: count("CONFORME"),
    noncompliant: count("NO CONFORME"),
    explained: checks.filter((check) => check.state === "NO CONFORME" && check.labels.length > 0)
      .length,
    not_observable: count("NO OBSERVABLE"),
    pending: count("PENDIENTE"),
  };
}

export function aSet(name: string, checks: CheckView[]): SetView {
  return { name, summary: summaryOf(checks), checks };
}

export function aReportView(): ReportView {
  const sets = [
    aSet("saludo", [aCheck("greeting_opens_the_channel", "CONFORME"), aCheck("greeting_echoes")]),
    aSet("errores", [
      aCheck("unsupported_protocol_uri_rejected", "NO CONFORME"),
      aCheck("unknown_operation_rejected", "NO OBSERVABLE"),
      aCheck("empty_uri_rejected"),
    ]),
  ];
  return {
    client: "/usr/bin/autofirma",
    kind: "autofirma",
    header: {
      os: "Linux",
      os_version: "6.8.0",
      client_version: "1.9.2",
      date: "2026-09-21",
    },
    summary: summaryOf(sets.flatMap((set) => set.checks)),
    sets,
    orphans: [],
  };
}

export function aSnapshot(overrides: Partial<Snapshot> = {}): Snapshot {
  return {
    client: {
      kind: "autofirma",
      binary: "/usr/bin/autofirma",
      profiles: (["rsa", "ec", "token"] as const).map((store) => ({
        store,
        launcher: `/tmp/aislado-${store}/launch-subject`,
        trust_root: "/usr/lib/Autofirma/Autofirma_ROOT.cer",
      })),
    },
    client_complaints: [],
    resolving_client: false,
    report_name: "af-linux-prueba",
    report: aReportView(),
    reports: [
      {
        name: "af-linux-prueba",
        kind: "autofirma",
        client: "/usr/bin/autofirma",
        client_version: "1.9.2",
        date: "2026-09-21",
        complaint: null,
      },
    ],
    running: null,
    queued: [],
    call: null,
    why_pending: {},
    ...overrides,
  };
}
