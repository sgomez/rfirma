import { type ReactNode, useCallback, useEffect, useState } from "react";
import { useParams } from "react-router";
import type { ReportView } from "../contract/ReportView";
import { LogDock } from "../log/LogDock";
import { useProgress } from "../session/progress";
import { useLive } from "../suite/live";
import { calendarDate, clientName } from "../words";
import { ReportBody } from "./ReportBody";
import { Counts, SummaryBar } from "./SummaryBar";
import { TranscriptModal } from "./TranscriptModal";
import { ValidateModal } from "./ValidateModal";

/** Cualquier informe en solo lectura, igual que el activo y en directo mientras la sesión lo ejecuta. */
export function ReportPage() {
  const name = useParams().name ?? "";
  const { snapshot, suite } = useLive();
  const live = snapshot !== null && snapshot.report_name === name && snapshot.report !== null;
  const [fetched, setFetched] = useState<ReportView | null>(null);
  const [complaint, setComplaint] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [transcript, setTranscript] = useState<string | null>(null);
  const [validating, setValidating] = useState(false);
  const progress = useProgress(snapshot, live);

  useEffect(() => {
    document.title = `${name} · Suite de conformidad`;
  }, [name]);

  const connected = snapshot !== null;
  useEffect(() => {
    if (live || !connected) return;
    let current = true;
    suite
      .reportView(name)
      .then((view) => {
        if (current) setFetched(view);
      })
      .catch((error: unknown) => {
        if (current) setComplaint(error instanceof Error ? error.message : String(error));
      });
    return () => {
      current = false;
    };
  }, [suite, name, live, connected]);

  const onSelect = useCallback((id: string) => setSelected(id), []);
  const view = live ? snapshot.report : fetched;

  if (!view) {
    return complaint ? (
      <div className="placeholder">
        <p className="complaint">{complaint}</p>
      </div>
    ) : (
      <p className="loading">Leyendo {name}…</p>
    );
  }

  return (
    <>
      <header className="report-head">
        <div className="report-title">
          <span className="eyebrow">
            Informe{live && <span className="live-badge">en directo</span>}
          </span>
          <h1 className="mono">{name}</h1>
          <dl className="meta">
            <Meta label="Cliente">
              {clientName(view.kind)} {view.header.client_version}
            </Meta>
            <Meta label="Binario">
              <code>{view.client}</code>
            </Meta>
            <Meta label="Sistema">
              {view.header.os} {view.header.os_version}
            </Meta>
            <Meta label="Almacén">
              <code>{view.header.store}</code>
            </Meta>
            <Meta label="Transporte">{view.header.transport}</Meta>
            <Meta label="Fecha">{calendarDate(view.header.date)}</Meta>
          </dl>
        </div>
        <div className="report-meter">
          <SummaryBar summary={view.summary} />
          <div className="report-meter-foot">
            <span className="run-total">
              <strong>{view.summary.total - view.summary.pending}</strong>
              <span className="muted">/{view.summary.total}</span>
            </span>
            <Counts summary={view.summary} />
          </div>
        </div>
        <button type="button" className="button" onClick={() => setValidating(true)}>
          Validar contra referencia…
        </button>
      </header>
      <ReportBody
        view={view}
        progress={progress}
        controls={null}
        selected={selected}
        onSelect={onSelect}
        onTranscript={setTranscript}
      />
      <LogDock
        check={selected}
        report={name}
        runningIds={live ? (snapshot.running?.ids ?? []) : []}
      />
      {transcript && (
        <TranscriptModal check={transcript} report={name} onClose={() => setTranscript(null)} />
      )}
      {validating && (
        <ValidateModal report={name} view={view} onClose={() => setValidating(false)} />
      )}
    </>
  );
}

function Meta({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div>
      <dt>{label}</dt>
      <dd>{children}</dd>
    </div>
  );
}
