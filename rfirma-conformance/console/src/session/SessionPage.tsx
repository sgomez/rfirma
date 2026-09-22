import { useCallback, useEffect, useMemo, useState } from "react";
import type { Assistance } from "../contract/Assistance";
import type { ReportView } from "../contract/ReportView";
import type { Tranches } from "../contract/Tranches";
import { LogDock } from "../log/LogDock";
import { ReportBody } from "../report/ReportBody";
import type { Controls } from "../report/SetSection";
import { Counts, SummaryBar } from "../report/SummaryBar";
import { TranscriptModal } from "../report/TranscriptModal";
import { ValidateModal } from "../report/ValidateModal";
import { useLive } from "../suite/live";
import { PlayIcon } from "../ui/icons";
import { askToNotify } from "../ui/notify";
import { focusedCheck, useShortcuts } from "../ui/shortcuts";
import { BatchBar } from "./BatchBar";
import { ClientStep } from "./ClientStep";
import { useBatch, useProgress } from "./progress";
import { ReportStep } from "./ReportStep";
import { Step } from "./Step";

export function SessionPage() {
  const { snapshot, suite, complain } = useLive();
  const [selected, setSelected] = useState<string | null>(null);
  const [follow, setFollow] = useState(true);
  const [transcript, setTranscript] = useState<string | null>(null);
  const [validating, setValidating] = useState(false);
  const progress = useProgress(snapshot, true);
  const batch = useBatch(snapshot);
  const head = snapshot?.running?.ids[0] ?? null;
  const view = snapshot?.report ?? null;

  useEffect(() => {
    document.title = "Sesión · Suite de conformidad";
  }, []);

  useEffect(() => {
    if (follow && head) setSelected(head);
  }, [follow, head]);

  const onSelect = useCallback((id: string) => {
    setSelected(id);
    setFollow(false);
  }, []);

  const controls = useMemo<Controls>(
    () => ({
      runCheck: (check) => void suite.run({ check }).catch(complain("No se pudo poner en cola")),
      runSet: (set, pendingOnly) =>
        void suite
          .run(pendingOnly ? { set, pending: true } : { set })
          .catch(complain("No se pudo poner en cola")),
    }),
    [suite, complain],
  );

  const runTranches = (tranches: Tranches) => {
    if (tranches === "all") askToNotify();
    void suite.run({ tranches }).catch(complain("No se pudo poner en cola"));
  };

  useShortcuts({
    p: view && view.summary.pending > 0 ? () => runTranches("all") : undefined,
    r: view
      ? () => {
          const check = focusedCheck();
          if (check) controls.runCheck(check);
        }
      : undefined,
  });

  if (!snapshot) return <p className="loading">Conectando con la suite…</p>;

  const busy = snapshot.running !== null || snapshot.queued.length > 0 || snapshot.resolving_client;
  const ready = snapshot.client !== null && view !== null;

  return (
    <>
      <div className="steps">
        <ClientStep snapshot={snapshot} busy={busy} />
        <ReportStep snapshot={snapshot} busy={busy} />
        <Step
          number={3}
          label="Ejecutar"
          done={false}
          disabled={!ready}
          summary={
            view ? (
              <span className="run-summary">
                <SummaryBar summary={view.summary} />
                <span className="run-total">
                  <strong>{view.summary.total - view.summary.pending}</strong>
                  <span className="muted">/{view.summary.total}</span>
                </span>
                <Counts summary={view.summary} />
              </span>
            ) : (
              <span className="muted">
                Con el cliente y el informe elegidos, ejecuta sus comprobaciones.
              </span>
            )
          }
          actions={
            view && (
              <>
                {TRANCHE_BUTTONS.map(([tranches, label, takes]) => {
                  const count = pendingIn(view, takes);
                  const all = tranches === "all";
                  return (
                    <button
                      key={tranches}
                      type="button"
                      className={all ? "button primary" : "button"}
                      onClick={() => runTranches(tranches)}
                      disabled={count === 0}
                      aria-keyshortcuts={all ? "p" : undefined}
                    >
                      {all && <PlayIcon />} {label} ({count}) {all && <kbd>p</kbd>}
                    </button>
                  );
                })}
                <span className="divider" aria-hidden="true" />
                <button type="button" className="button ghost" onClick={() => setValidating(true)}>
                  Comparar con la referencia…
                </button>
              </>
            )
          }
        />
      </div>
      {batch && <BatchBar batch={batch} question={snapshot.question} />}
      {view ? (
        <ReportBody
          view={view}
          progress={progress}
          controls={ready ? controls : null}
          selected={selected}
          onSelect={onSelect}
          onTranscript={setTranscript}
        />
      ) : (
        <div className="placeholder">
          <p>Elige un cliente y un informe: aquí aparecerán sus conjuntos y comprobaciones.</p>
        </div>
      )}
      <LogDock
        check={selected}
        runningIds={snapshot.running?.ids ?? []}
        follow={{ on: follow, set: setFollow }}
      />
      {transcript && <TranscriptModal check={transcript} onClose={() => setTranscript(null)} />}
      {validating && view && snapshot.report_name && (
        <ValidateModal
          report={snapshot.report_name}
          view={view}
          onClose={() => setValidating(false)}
        />
      )}
    </>
  );
}

const TRANCHE_BUTTONS: [Tranches, string, (assistance: Assistance) => boolean][] = [
  ["unattended", "Solo las automáticas", (assistance) => assistance === "none"],
  ["attended", "Las que te necesitan", (assistance) => assistance !== "none"],
  ["all", "Ejecutar todas", () => true],
];

function pendingIn(view: ReportView, takes: (assistance: Assistance) => boolean): number {
  return view.sets
    .flatMap((set) => set.checks)
    .filter((check) => check.state === "PENDIENTE" && takes(check.assistance ?? "none")).length;
}
