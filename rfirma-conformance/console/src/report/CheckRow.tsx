import { memo, type ReactNode, useEffect, useRef, useState } from "react";
import type { CheckView } from "../contract/CheckView";
import type { ClientKind } from "../contract/ClientKind";
import type { KnownBug } from "../contract/KnownBug";
import { Elapsed } from "../ui/Elapsed";
import {
  type Activity,
  ActivityIcon,
  CheckMark,
  CopyIcon,
  PlayIcon,
  ResultIcon,
} from "../ui/icons";
import {
  assistanceName,
  bugLabel,
  calendarDate,
  DEPRECATED_LABEL,
  DEPRECATED_REASON,
  duration,
  isAnExpectedFailure,
  resultTone,
} from "../words";
import type { Controls } from "./SetSection";

interface CheckRowProps {
  check: CheckView;
  kind: ClientKind;
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
  kind,
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
  const expected = isAnExpectedFailure(check, kind) || isADeprecatedFailure(check);
  return (
    <li
      className="check"
      data-check={check.id}
      data-tone={resultTone[check.state]}
      data-expected={expected || undefined}
      data-activity={activity ?? undefined}
      data-selected={selected || undefined}
      data-settled={settled || undefined}
    >
      <div className="check-line">
        {/* biome-ignore lint/a11y/useSemanticElements: un <button> no deja seleccionar el nombre */}
        <div
          role="button"
          tabIndex={0}
          className="check-toggle"
          data-check-row
          aria-expanded={expanded}
          aria-controls={detailId}
          onClick={() => {
            if (!endsATextSelection()) onToggle(check.id);
          }}
          onKeyDown={(event) => {
            if (event.target !== event.currentTarget) return;
            if (event.key !== "Enter" && event.key !== " ") return;
            event.preventDefault();
            onToggle(check.id);
          }}
        >
          <span className="check-icon">
            {activity ? <ActivityIcon activity={activity} /> : <ResultIcon result={check.state} />}
          </span>
          <span className="check-name">
            <span className="check-id">{check.id}</span>
            {check.bug && <BugTag bug={check.bug} />}
            {check.deprecated && (
              <span className="tag tag-deprecated" title={DEPRECATED_REASON}>
                {DEPRECATED_LABEL}
              </span>
            )}
          </span>
          <span className="check-chapter">cap. {check.chapter}</span>
          <span className="check-status">
            <Status check={check} activity={activity} runningSince={runningSince} />
          </span>
        </div>
        <CopyId id={check.id} />
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
          expected={expected}
          activity={activity}
          whyPending={whyPending}
          onTranscript={onTranscript}
        />
      )}
    </li>
  );
});

function endsATextSelection(): boolean {
  const selection = window.getSelection();
  return selection !== null && !selection.isCollapsed && selection.toString() !== "";
}

function isADeprecatedFailure(check: CheckView): boolean {
  return check.deprecated && check.state === "NO CONFORME";
}

function BugTag({ bug }: { bug: KnownBug }) {
  return (
    <span className="tag tag-bug" title={`${bug.id}: ${bug.title}`}>
      {bugLabel(bug)}
    </span>
  );
}

function CopyId({ id }: { id: string }) {
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), 1200);
    return () => window.clearTimeout(timer);
  }, [copied]);
  return (
    <button
      type="button"
      className="copy-id"
      data-copied={copied || undefined}
      onClick={() => void navigator.clipboard.writeText(id).then(() => setCopied(true))}
      aria-label={`Copiar ${id}`}
      title={copied ? "Copiado" : "Copiar el nombre"}
    >
      {copied ? <CheckMark /> : <CopyIcon />}
    </button>
  );
}

function Status({
  check,
  activity,
  runningSince,
}: Pick<CheckRowProps, "check" | "activity" | "runningSince">) {
  if (activity === "queued") return <span className="tag">en cola</span>;
  if (activity === "waiting")
    return <span className="tag tag-accent">esperando a que des paso</span>;
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
  expected,
  activity,
  whyPending,
  onTranscript,
}: {
  id: string;
  check: CheckView;
  expected: boolean;
  activity: Activity | null;
  whyPending: string | null;
  onTranscript: (id: string) => void;
}) {
  const shownResult =
    activity === "running"
      ? "en curso"
      : activity === "waiting"
        ? "esperando a que des paso"
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
        {check.assistance && (
          <Field label="Qué te pide">
            {assistanceName[check.assistance]} · almacén <code>{check.store}</code>
          </Field>
        )}
        {check.warning && (
          <Field label="Antes de empezar" tone="notice">
            {check.warning}
          </Field>
        )}
        {check.bug && (
          <Field label="Bug conocido">
            {bugLabel(check.bug)} · <code>{check.bug.id}</code> {check.bug.title}
          </Field>
        )}
        {check.deprecated && <Field label={DEPRECATED_LABEL}>{DEPRECATED_REASON}</Field>}
        {check.observation && <Field label="Qué pasó">{check.observation}</Field>}
        <Field label="Resultado">
          <span className={`result-label tone-${resultTone[check.state]}`}>
            <ResultIcon result={check.state} size={12} decorative />
            {shownResult ?? check.state}
          </span>
          {expected && !shownResult && (
            <span className="muted">
              {check.deprecated ? " · no cuenta como fallo" : " · esperado por el bug"}
            </span>
          )}
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
