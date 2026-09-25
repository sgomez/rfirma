import type { CheckView } from "../contract/CheckView";
import type { KnownBug } from "../contract/KnownBug";
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
    question: null,
    assistance: "none",
    store: "rsa",
    bug: null,
    deprecated: false,
    state,
    observation: state === "PENDIENTE" ? null : "el trámite se completó",
    date: state === "PENDIENTE" ? null : "2026-09-21",
    duration_ms: state === "PENDIENTE" ? null : 3500,
  };
}

export function aKnownBug(master: KnownBug["master"] = "present"): KnownBug {
  return { id: "BUG-15", title: "Ausencia de validación de cop en signandsave", master };
}

export function withABug(view: ReportView, id: string, bug: KnownBug): ReportView {
  return {
    ...view,
    sets: view.sets.map((set) => ({
      ...set,
      checks: set.checks.map((check) => (check.id === id ? { ...check, bug } : check)),
    })),
  };
}

export function withADeprecatedFormat(view: ReportView, id: string): ReportView {
  const sets = view.sets.map((set) =>
    aSet(
      set.name,
      set.checks.map((check) => (check.id === id ? { ...check, deprecated: true } : check)),
    ),
  );
  return { ...view, sets, summary: summaryOf(sets.flatMap((set) => set.checks)) };
}

function summaryOf(checks: CheckView[]): Summary {
  const count = (state: ResultName) => checks.filter((check) => check.state === state).length;
  const deprecated = checks.filter(
    (check) => check.deprecated && check.state === "NO CONFORME",
  ).length;
  return {
    total: checks.length,
    compliant: count("CONFORME"),
    noncompliant: count("NO CONFORME") - deprecated,
    deprecated,
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
    question: null,
    why_pending: {},
    ...overrides,
  };
}
