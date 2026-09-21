import { memo, type ReactNode, useEffect, useRef, useState } from "react";
import type { CheckView } from "../contract/CheckView";
import { Elapsed } from "../ui/Elapsed";
import { type Activity, ActivityIcon, PlayIcon, ResultIcon } from "../ui/icons";
import { calendarDate, duration, resultTone } from "../words";
import type { Controls } from "./SetSection";

interface CheckRowProps {
  check: CheckView;
  activity: Activity | null;
  whyPending: string | null;
  runningSince: number | null;
  expanded: boolean;
  selected: boolean;
  onToggle: (id: string) => void;
  controls: Controls | null;
  onTranscript: (id: string) => void;
}

export const CheckRow = memo(function CheckRow({
  check,
  activity,
  whyPending,
  runningSince,
  expanded,
  selected,
  onToggle,
  controls,
  onTranscript,
}: CheckRowProps) {
  const settled = useJustSettled(check.state);
  const detailId = `detail-${check.id}`;
  return (
    <li
      className="check"
      data-check={check.id}
      data-tone={resultTone[check.state]}
      data-activity={activity ?? undefined}
      data-selected={selected || undefined}
      data-settled={settled || undefined}
    >
      <div className="check-line">
        <button
          type="button"
          className="check-toggle"
          data-check-row
          aria-expanded={expanded}
          aria-controls={detailId}
          onClick={() => onToggle(check.id)}
        >
          <span className="check-icon">
            {activity ? <ActivityIcon activity={activity} /> : <ResultIcon result={check.state} />}
          </span>
          <span className="check-id">{check.id}</span>
          <span className="check-chapter">cap. {check.chapter}</span>
          <span className="check-status">
            <Status check={check} activity={activity} runningSince={runningSince} />
          </span>
        </button>
        {controls && (
          <button
            type="button"
            className="run-one"
            disabled={activity !== null}
            onClick={() => controls.runCheck(check.id)}
            aria-label={`Ejecutar ${check.id}`}
            title={check.state === "PENDIENTE" ? "Ejecutar (r)" : "Repetir (r)"}
          >
            <PlayIcon />
          </button>
        )}
      </div>
      {expanded && (
        <CheckDetail
          id={detailId}
          check={check}
          activity={activity}
          whyPending={whyPending}
          onTranscript={onTranscript}
        />
      )}
    </li>
  );
});

function Status({
  check,
  activity,
  runningSince,
}: Pick<CheckRowProps, "check" | "activity" | "runningSince">) {
  if (activity === "queued") return <span className="tag">en cola</span>;
  if (activity === "asking") return <span className="tag tag-accent">esperando tu respuesta</span>;
  if (activity === "running") {
    return (
      <span className="tag tag-accent">
        en curso {runningSince !== null && <Elapsed since={runningSince} />}
      </span>
    );
  }
  if (check.duration_ms !== null)
    return <span className="muted">{duration(check.duration_ms)}</span>;
  return null;
}

function CheckDetail({
  id,
  check,
  activity,
  whyPending,
  onTranscript,
}: {
  id: string;
  check: CheckView;
  activity: Activity | null;
  whyPending: string | null;
  onTranscript: (id: string) => void;
}) {
  const shownResult =
    activity === "running"
      ? "en curso"
      : activity === "asking"
        ? "esperando tu respuesta"
        : activity === "queued"
          ? "en cola"
          : null;
  return (
    <div className="check-detail" id={id}>
      <dl className="fields">
        <Field label="Qué se exige">{check.statement}</Field>
        <Field label="Fuente">
          <code>{check.citation}</code>
        </Field>
        {check.warning && (
          <Field label="Antes de empezar" tone="notice">
            {check.warning}
          </Field>
        )}
        {check.question && <Field label="Te preguntaremos">{check.question}</Field>}
        {check.observation && <Field label="Qué pasó">{check.observation}</Field>}
        <Field label="Resultado">
          <span className={`result-label tone-${resultTone[check.state]}`}>
            <ResultIcon result={check.state} size={12} decorative />
            {shownResult ?? check.state}
          </span>
          {check.date && <span className="muted"> · {calendarDate(check.date)}</span>}
          {check.duration_ms !== null && (
            <span className="muted"> · {duration(check.duration_ms)}</span>
          )}
        </Field>
        {whyPending && <Field label="Pendiente porque">{whyPending}</Field>}
      </dl>
      <div className="detail-actions">
        <button type="button" className="button small" onClick={() => onTranscript(check.id)}>
          Ver tramas
        </button>
      </div>
    </div>
  );
}

function Field({ label, tone, children }: { label: string; tone?: "notice"; children: ReactNode }) {
  return (
    <div className={`field${tone ? ` field-${tone}` : ""}`}>
      <dt>{label}</dt>
      <dd>{children}</dd>
    </div>
  );
}

function useJustSettled(state: CheckView["state"]): boolean {
  const previous = useRef(state);
  const [settled, setSettled] = useState(false);
  useEffect(() => {
    if (previous.current === state) return;
    previous.current = state;
    setSettled(true);
    const timer = window.setTimeout(() => setSettled(false), 1200);
    return () => window.clearTimeout(timer);
  }, [state]);
  return settled;
}
