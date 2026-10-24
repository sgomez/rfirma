import { useEffect, useId, useState } from "react";
import type { CatalogueSet } from "../contract/CatalogueSet";
import type { SelectionResolution } from "../contract/SelectionResolution";
import { useLive } from "../suite/live";

const NOTHING_NAMED: SelectionResolution = {
  ids: [],
  greetings_added: [],
  unknown: [],
  out_of_scope: [],
  sharing_an_errand: [],
};

interface Preview {
  text: string;
  resolution: SelectionResolution;
}

/** Lo que la sesión resuelve de `text` dentro de `scope`; `null` mientras llega la respuesta a lo último escrito. */
export function useScopePreview(text: string, scope?: string | null): SelectionResolution | null {
  const { suite, complain } = useLive();
  const [preview, setPreview] = useState<Preview | null>(null);

  useEffect(() => {
    if (text.trim() === "") {
      setPreview({ text, resolution: NOTHING_NAMED });
      return;
    }
    let current = true;
    suite
      .selection(text, scope ?? undefined)
      .then((resolution) => current && setPreview({ text, resolution }))
      .catch(complain("No se pudo resolver la selección"));
    return () => {
      current = false;
    };
  }, [suite, complain, text, scope]);

  return preview?.text === text ? preview.resolution : null;
}

function isUnder(set: string, name: string): boolean {
  return set === name || set.startsWith(`${name}.`);
}

function rewritten(
  sets: CatalogueSet[],
  marked: Set<string>,
  resolution: SelectionResolution,
  written: string,
): string | null {
  const whole = (set: CatalogueSet) => set.checks.every((id) => marked.has(id));
  const kept = written.split(/[\s,]+/).filter((name) => {
    const under = sets.filter((set) => isUnder(set.name, name));
    return under.length > 0 && under.every(whole);
  });
  const names = sets
    .filter((set) => !kept.some((name) => isUnder(set.name, name)))
    .flatMap((set) => (whole(set) ? [set.name] : set.checks.filter((id) => marked.has(id))));
  const text = [...new Set([...kept, ...names, ...resolution.unknown])].join(" ");
  return text === "" ? null : text;
}

export function ScopeField({
  value,
  resolution,
  onChange,
}: {
  value: string | null;
  resolution: SelectionResolution | null;
  onChange: (scope: string | null) => void;
}) {
  const { suite, complain } = useLive();
  const [sets, setSets] = useState<CatalogueSet[]>([]);
  const previewId = useId();

  useEffect(() => {
    suite.catalogue().then(setSets).catch(complain("No se pudo leer el catálogo"));
  }, [suite, complain]);

  const marked = new Set(resolution?.ids.filter((id) => !resolution.greetings_added.includes(id)));

  const toggle = (ids: string[]) => {
    if (!resolution) return;
    const next = new Set(marked);
    const all = ids.every((id) => next.has(id));
    for (const id of ids) {
      if (all) next.delete(id);
      else next.add(id);
    }
    onChange(rewritten(sets, next, resolution, value ?? ""));
  };

  return (
    <div className="form-row scope-field">
      <label className="form-row">
        <span className="field-label">Alcance</span>
        <input
          className="input"
          value={value ?? ""}
          placeholder="ids o conjuntos; vacío mide el catálogo entero"
          aria-describedby={previewId}
          onChange={(event) =>
            onChange(event.target.value.trim() === "" ? null : event.target.value)
          }
          spellCheck={false}
        />
      </label>
      <p id={previewId} className="muted" role="status">
        <ScopePreview resolution={resolution} whole={value === null} />
      </p>
      <ul className="scope-tree" aria-label="Conjuntos y comprobaciones">
        {sets.map((set) => {
          const count = set.checks.filter((id) => marked.has(id)).length;
          return (
            <li key={set.name}>
              <label>
                <input
                  type="checkbox"
                  aria-label={`Marcar ${set.name}`}
                  checked={count === set.checks.length}
                  disabled={resolution === null}
                  ref={(box) => {
                    if (box) box.indeterminate = count > 0 && count < set.checks.length;
                  }}
                  onChange={() => toggle(set.checks)}
                />
                <strong>{set.name}</strong>
              </label>
              <ul>
                {set.checks.map((id) => (
                  <li key={id}>
                    <label>
                      <input
                        type="checkbox"
                        aria-label={`Marcar ${id}`}
                        checked={marked.has(id)}
                        disabled={resolution === null}
                        onChange={() => toggle([id])}
                      />
                      <span className="mono">{id}</span>
                    </label>
                  </li>
                ))}
              </ul>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function ScopePreview({
  resolution,
  whole,
}: {
  resolution: SelectionResolution | null;
  whole: boolean;
}) {
  if (whole) return <>Sin alcance: se mide el catálogo entero.</>;
  if (!resolution) return <>Resolviendo…</>;
  return (
    <>
      Resuelve {resolution.ids.length} comprobaciones.
      {resolution.greetings_added.length > 0 &&
        ` Añade los saludos: ${resolution.greetings_added.join(", ")}.`}
      {resolution.unknown.length > 0 && (
        <span className="complaint"> No casan: {resolution.unknown.join(", ")}.</span>
      )}
    </>
  );
}
