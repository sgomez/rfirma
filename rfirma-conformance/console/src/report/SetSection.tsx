import { memo } from "react";
import type { CheckView } from "../contract/CheckView";
import type { ClientKind } from "../contract/ClientKind";
import type { SetView } from "../contract/SetView";
import type { Activity } from "../ui/icons";
import { Chevron, PlayIcon } from "../ui/icons";
import { CheckRow } from "./CheckRow";
import { Counts, SummaryBar } from "./SummaryBar";

export interface Progress {
  activityOf: (id: string) => Activity | null;
  whyPending: (id: string) => string | null;
  runningSince: number | null;
}

export interface Controls {
  runCheck: (id: string) => void;
  runSet: (set: string, pendingOnly: boolean) => void;
}

interface SetSectionProps {
  set: SetView;
  checks: CheckView[];
  kind: ClientKind;
  folded: boolean;
  onToggle: (name: string) => void;
  expanded: ReadonlySet<string>;
  onToggleCheck: (id: string) => void;
  selected: string | null;
  progress: Progress;
  controls: Controls | null;
  onTranscript: (id: string) => void;
}

export const SetSection = memo(function SetSection({
  set,
  checks,
  kind,
  folded,
  onToggle,
  expanded,
  onToggleCheck,
  selected,
  progress,
  controls,
  onTranscript,
}: SetSectionProps) {
  const listId = `set-${set.name}`;
  const busy = set.checks.some((check) => progress.activityOf(check.id) !== null);
  return (
    <section className={`set${busy ? " is-busy" : ""}`} aria-labelledby={`${listId}-title`}>
      <header className="set-bar">
        <h2 className="set-title" id={`${listId}-title`}>
          <button
            type="button"
            className="set-toggle"
            aria-expanded={!folded}
            aria-controls={listId}
            onClick={() => onToggle(set.name)}
          >
            <Chevron open={!folded} />
            <span className="set-name">{set.name}</span>
          </button>
        </h2>
        <Counts summary={set.summary} />
        <SummaryBar summary={set.summary} compact />
        {controls && (
          <div className="set-actions">
            <button
              type="button"
              className="button ghost small"
              disabled={set.summary.pending === 0}
              onClick={() => controls.runSet(set.name, true)}
              aria-label={`Ejecutar las pendientes de ${set.name}`}
            >
              <PlayIcon /> Pendientes
              <span className="count">{set.summary.pending}</span>
            </button>
            <button
              type="button"
              className="button ghost small"
              onClick={() => controls.runSet(set.name, false)}
              aria-label={`Ejecutar todo ${set.name}`}
            >
              Todo
            </button>
          </div>
        )}
      </header>
      {!folded && (
        <ul className="checks" id={listId}>
          {checks.map((check) => (
            <CheckRow
              key={check.id}
              check={check}
              kind={kind}
              activity={progress.activityOf(check.id)}
              whyPending={progress.whyPending(check.id)}
              runningSince={progress.runningSince}
              expanded={expanded.has(check.id)}
              selected={selected === check.id}
              onToggle={onToggleCheck}
              controls={controls}
              onTranscript={onTranscript}
            />
          ))}
        </ul>
      )}
    </section>
  );
});
