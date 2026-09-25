import { type KeyboardEvent, useCallback, useMemo, useState } from "react";
import type { ReportView } from "../contract/ReportView";
import type { ResultName } from "../contract/ResultName";
import { ResultIcon } from "../ui/icons";
import { moveAmongChecks, useShortcuts } from "../ui/shortcuts";
import { RESULTS, rowsWith } from "../words";
import { type Controls, type Progress, SetSection } from "./SetSection";

export interface ReportBodyProps {
  view: ReportView;
  progress: Progress;
  controls: Controls | null;
  selected: string | null;
  onSelect: (id: string) => void;
  onTranscript: (id: string) => void;
}

export function ReportBody({
  view,
  progress,
  controls,
  selected,
  onSelect,
  onTranscript,
}: ReportBodyProps) {
  const [folded, setFolded] = useState<ReadonlySet<string>>(new Set());
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const [shown, setShown] = useState<ReadonlySet<ResultName>>(new Set(RESULTS));

  const foldAll = useCallback(() => setFolded(new Set(view.sets.map((set) => set.name))), [view]);
  const unfoldAll = useCallback(() => setFolded(new Set()), []);
  const toggleSet = useCallback((name: string) => setFolded((was) => toggled(was, name)), []);
  const toggleCheck = useCallback(
    (id: string) => {
      setExpanded((was) => toggled(was, id));
      onSelect(id);
    },
    [onSelect],
  );

  useShortcuts({
    "[": foldAll,
    "]": unfoldAll,
    j: () => moveAmongChecks(1),
    k: () => moveAmongChecks(-1),
  });

  const onArrows = (event: KeyboardEvent) => {
    if (!(event.target instanceof HTMLElement) || !event.target.matches("[data-check-row]")) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      moveAmongChecks(event.key === "ArrowDown" ? 1 : -1);
    }
  };

  const visibleSets = useMemo(
    () =>
      view.sets
        .map((set) => ({
          set,
          checks: set.checks.filter(
            (check) => shown.has(check.state) || progress.activityOf(check.id) !== null,
          ),
        }))
        .filter(({ checks }) => checks.length > 0),
    [view, shown, progress],
  );

  const allFolded = folded.size === view.sets.length;

  return (
    <section className="report-body" aria-label="Conjuntos">
      <div className="sets-toolbar">
        <fieldset className="filter" aria-label="Mostrar por resultado">
          {RESULTS.map((result) => (
            <button
              key={result}
              type="button"
              className="chip"
              aria-pressed={shown.has(result)}
              onClick={() => setShown((was) => toggled(was, result))}
            >
              <ResultIcon result={result} size={12} decorative />
              <span>{result}</span>
              <span className="count">{rowsWith(view.summary, result)}</span>
            </button>
          ))}
        </fieldset>
        <div className="toolbar-actions">
          <button
            type="button"
            className="button ghost small"
            onClick={foldAll}
            disabled={allFolded}
            aria-keyshortcuts="["
          >
            Plegar todo <kbd>[</kbd>
          </button>
          <button
            type="button"
            className="button ghost small"
            onClick={unfoldAll}
            disabled={folded.size === 0}
            aria-keyshortcuts="]"
          >
            Desplegar todo <kbd>]</kbd>
          </button>
        </div>
      </div>
      {/* biome-ignore lint/a11y/noStaticElementInteractions: delegates arrow keys to the check rows inside */}
      <div className="sets" onKeyDown={onArrows}>
        {visibleSets.map(({ set, checks }) => (
          <SetSection
            key={set.name}
            set={set}
            checks={checks}
            kind={view.kind}
            folded={folded.has(set.name)}
            onToggle={toggleSet}
            expanded={expanded}
            onToggleCheck={toggleCheck}
            selected={selected}
            progress={progress}
            controls={controls}
            onTranscript={onTranscript}
          />
        ))}
        {visibleSets.length === 0 && (
          <p className="empty">Ninguna comprobación tiene los resultados elegidos.</p>
        )}
      </div>
      {view.orphans.length > 0 && <Orphans orphans={view.orphans} />}
    </section>
  );
}

function Orphans({ orphans }: { orphans: ReportView["orphans"] }) {
  return (
    <section className="orphans" aria-label="Huérfanas">
      <h3>Huérfanas</h3>
      <p className="hint">
        El informe las guarda, pero el catálogo ya no las tiene: no cuentan en los recuentos y se
        podan la próxima vez que se escriba el informe.
      </p>
      <ul>
        {orphans.map((orphan) => (
          <li key={orphan.id}>
            <ResultIcon result={orphan.state} size={12} decorative />
            <code>{orphan.id}</code>
            <span>{orphan.state}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}

function toggled<T>(set: ReadonlySet<T>, item: T): ReadonlySet<T> {
  const next = new Set(set);
  if (next.has(item)) next.delete(item);
  else next.add(item);
  return next;
}
