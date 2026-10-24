import { type KeyboardEvent, useCallback, useMemo, useRef, useState } from "react";
import type { ReportView } from "../contract/ReportView";
import type { ResultName } from "../contract/ResultName";
import type { SelectionResolution } from "../contract/SelectionResolution";
import { useScopePreview } from "../session/ScopeField";
import { PlayIcon, ResultIcon } from "../ui/icons";
import { Modal } from "../ui/Modal";
import { isListOf, useStoredState } from "../ui/preferences";
import { moveAmongChecks, useShortcuts } from "../ui/shortcuts";
import { countOf, RESULTS } from "../words";
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
  const [shownResults, setShownResults] = useStoredState<readonly ResultName[]>(
    "results",
    RESULTS,
    isListOf(RESULTS),
  );
  const shown = useMemo<ReadonlySet<ResultName>>(() => new Set(shownResults), [shownResults]);
  const setShown = useCallback(
    (change: (was: ReadonlySet<ResultName>) => ReadonlySet<ResultName>) =>
      setShownResults((was) => [...change(new Set(was))]),
    [setShownResults],
  );
  const [text, setText] = useState("");
  const [resetting, setResetting] = useState(false);
  const field = useRef<HTMLInputElement>(null);
  const resolution = useLastResolution(text, view.header.scope);
  const named = useMemo(() => namedBy(text, resolution), [text, resolution]);

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

  const seen = useMemo(
    () =>
      view.sets
        .flatMap((set) => set.checks)
        .filter((check) => (named === null || named.has(check.id)) && shown.has(check.state))
        .map((check) => check.id),
    [view, named, shown],
  );
  const seenSelection = shown.size === RESULTS.length ? text.trim() : seen.join(" ");
  const refused = text.trim() !== "" && resolution !== null && !isRunnable(resolution);
  const acts = controls !== null && seen.length > 0 && !refused;

  useShortcuts({
    "[": foldAll,
    "]": unfoldAll,
    j: () => moveAmongChecks(1),
    k: () => moveAmongChecks(-1),
    "/": () => field.current?.focus(),
    e: acts && controls ? () => controls.run(seenSelection, "reuse") : undefined,
    E: acts && controls ? () => controls.run(seenSelection, "relaunch") : undefined,
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
            (check) =>
              (named === null || named.has(check.id)) &&
              (shown.has(check.state) || progress.activityOf(check.id) !== null),
          ),
        }))
        .filter(({ checks }) => checks.length > 0),
    [view, named, shown, progress],
  );

  const allFolded = folded.size === view.sets.length;

  return (
    <section className="report-body" aria-label="Conjuntos">
      <div className="sets-toolbar">
        <label className="selection-field">
          <input
            ref={field}
            type="search"
            className="input"
            aria-label="Selección"
            aria-keyshortcuts="/"
            placeholder="ids o conjuntos  /"
            value={text}
            onChange={(event) => setText(event.target.value)}
            spellCheck={false}
          />
          {refused && resolution && <Refusal resolution={resolution} />}
        </label>
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
              <span className="count">{countOf(view.summary, result)}</span>
            </button>
          ))}
        </fieldset>
        <div className="toolbar-actions">
          {controls && (
            <>
              <button
                type="button"
                className="button small"
                disabled={!acts}
                onClick={() => controls.run(seenSelection, "reuse")}
                aria-keyshortcuts="e"
                title="Reutiliza los trámites guardados"
              >
                <PlayIcon /> Ejecutar lo visible <kbd>e</kbd>
              </button>
              <button
                type="button"
                className="button small"
                disabled={!acts}
                onClick={() => controls.run(seenSelection, "relaunch")}
                aria-keyshortcuts="Shift+E"
                title="Abre el cliente otra vez y sustituye los trámites guardados"
              >
                Relanzar lo visible <kbd>⇧E</kbd>
              </button>
              <button
                type="button"
                className="button ghost small"
                disabled={!acts}
                onClick={() => setResetting(true)}
              >
                Reiniciar lo visible…
              </button>
            </>
          )}
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
      {resetting && controls && (
        <ResetModal
          selection={seenSelection}
          count={seen.length}
          scope={view.header.scope}
          onReset={() => {
            controls.reset(seenSelection);
            setResetting(false);
          }}
          onClose={() => setResetting(false)}
        />
      )}
    </section>
  );
}

/** La última resolución llegada de `text`, para no deshacer el filtro mientras llega la siguiente. */
function useLastResolution(text: string, scope: string | null): SelectionResolution | null {
  const resolved = useScopePreview(text, scope);
  const last = useRef<SelectionResolution | null>(null);
  if (resolved) last.current = resolved;
  return last.current;
}

function namedBy(text: string, resolution: SelectionResolution | null): ReadonlySet<string> | null {
  if (text.trim() === "" || resolution === null) return null;
  return new Set(resolution.ids.filter((id) => !resolution.greetings_added.includes(id)));
}

function isRunnable(resolution: SelectionResolution): boolean {
  return resolution.unknown.length === 0 && resolution.out_of_scope.length === 0;
}

function Refusal({ resolution }: { resolution: SelectionResolution }) {
  return (
    <span className="complaint" role="status">
      {resolution.unknown.length > 0 && `No casan: ${resolution.unknown.join(", ")}. `}
      {resolution.out_of_scope.length > 0 &&
        `Fuera del alcance: ${resolution.out_of_scope.join(", ")}.`}
    </span>
  );
}

function ResetModal({
  selection,
  count,
  scope,
  onReset,
  onClose,
}: {
  selection: string;
  count: number;
  scope: string | null;
  onReset: () => void;
  onClose: () => void;
}) {
  const preview = useScopePreview(selection, scope);
  const sharing = preview?.sharing_an_errand ?? [];
  return (
    <Modal
      title="Reiniciar lo visible"
      onClose={onClose}
      actions={
        <>
          <button type="button" className="button ghost" onClick={onClose}>
            Cancelar
          </button>
          <button
            type="button"
            className="button danger"
            onClick={onReset}
            disabled={preview === null}
          >
            Reiniciar
          </button>
        </>
      }
    >
      <p>Vuelven a PENDIENTE {count} comprobaciones y se olvidan sus trámites.</p>
      {sharing.length > 0 && (
        <>
          <p>
            Estas otras comparten trámite con ellas: conservan su resultado, pero se relanzarán
            cuando se ejecuten.
          </p>
          <ul>
            {sharing.map((id) => (
              <li key={id}>
                <code>{id}</code>
              </li>
            ))}
          </ul>
        </>
      )}
    </Modal>
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
