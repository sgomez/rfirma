import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { PendingByTranches } from "../contract/PendingByTranches";
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
import { isBoolean, useStoredState } from "../ui/preferences";
import { focusedCheck, useShortcuts } from "../ui/shortcuts";
import { BatchBar } from "./BatchBar";
import { ClientStep } from "./ClientStep";
import { useProgress } from "./progress";
import { ReportStep } from "./ReportStep";
import { Step } from "./Step";

export function SessionPage() {
  const { snapshot, suite, complain } = useLive();
  const [selected, setSelected] = useState<string | null>(null);
  const [following, setFollowing] = useStoredState("follow", true, isBoolean);
  const [suspended, setSuspended] = useState(false);
  const [transcript, setTranscript] = useState<string | null>(null);
  const [validating, setValidating] = useState(false);
  const progress = useProgress(snapshot, true);
  const head = snapshot?.running?.ids[0] ?? null;
  const view = snapshot?.report ?? null;

  useEffect(() => {
    document.title = "Sesión · Suite de conformidad";
  }, []);

  const batchOn = snapshot?.batch != null;
  const wasBatchOn = useRef(false);
  useEffect(() => {
    if (batchOn && !wasBatchOn.current) {
      setFollowing(true);
      setSuspended(false);
    }
    wasBatchOn.current = batchOn;
  }, [batchOn, setFollowing]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: la siguiente comprobación en curso retoma el seguimiento
  useEffect(() => setSuspended(false), [head]);

  useEffect(() => {
    if (following && !suspended && head) setSelected(head);
  }, [following, suspended, head]);

  const setFollow = useCallback(
    (on: boolean) => {
      setFollowing(on);
      setSuspended(false);
    },
    [setFollowing],
  );

  const onSelect = useCallback((id: string) => {
    setSelected(id);
    setSuspended(true);
  }, []);

  const controls = useMemo<Controls>(
    () => ({
      run: (selection, mode) =>
        void suite.run({ selection, mode }).catch(complain("No se pudo poner en cola")),
      reset: (selection) => void suite.reset(selection).catch(complain("No se pudo resetear")),
      runCheck: (check) =>
        void suite
          .run({ selection: check, mode: isPending(view, check) ? "reuse" : "relaunch" })
          .catch(complain("No se pudo poner en cola")),
      resetCheck: (check) => void suite.reset(check).catch(complain("No se pudo resetear")),
      runSet: (set, pendingOnly) =>
        void suite
          .run({ selection: set, mode: pendingOnly ? "reuse" : "relaunch" })
          .catch(complain("No se pudo poner en cola")),
    }),
    [suite, complain, view],
  );

  const runTranches = (tranches: Tranches) => {
    if (tranches === "all") askToNotify();
    void suite.run({ selection: "", tranches }).catch(complain("No se pudo poner en cola"));
  };

  useShortcuts({
    p: (snapshot?.pending_by_tranches?.all ?? 0) > 0 ? () => runTranches("all") : undefined,
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
  const current = head ?? snapshot.queued[0] ?? "";

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
                {TRANCHE_BUTTONS.map(([tranches, label]) => {
                  const count = snapshot.pending_by_tranches?.[tranches] ?? 0;
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
      {snapshot.batch && (
        <BatchBar
          batch={snapshot.batch}
          set={setOf(view, current)}
          current={current}
          instructions={warningOf(view, head)}
        />
      )}
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
        follow={{ on: following, set: setFollow }}
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

const TRANCHE_BUTTONS: [Tranches & keyof PendingByTranches, string][] = [
  ["unattended", "Solo las automáticas"],
  ["attended", "Las que te necesitan"],
  ["all", "Ejecutar todas"],
];

function setOf(view: ReportView | null, id: string): string | null {
  return view?.sets.find((set) => set.checks.some((check) => check.id === id))?.name ?? null;
}

function isPending(view: ReportView | null, id: string): boolean {
  const check = view?.sets.flatMap((set) => set.checks).find((check) => check.id === id);
  return check?.state === "PENDIENTE";
}

function warningOf(view: ReportView | null, id: string | null): string | null {
  const check = view?.sets.flatMap((set) => set.checks).find((check) => check.id === id);
  return check && check.assistance !== "none" ? check.warning : null;
}
