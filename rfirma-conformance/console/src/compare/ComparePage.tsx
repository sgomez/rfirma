import { useEffect, useId, useState } from "react";
import { useSearchParams } from "react-router";
import type { Comparison } from "../contract/Comparison";
import type { ReportEntry } from "../contract/ReportEntry";
import type { Row } from "../contract/Row";
import type { Side } from "../contract/Side";
import { useLive } from "../suite/live";
import { ResultIcon } from "../ui/icons";
import { calendarDate, clientName, resultTone } from "../words";

/** Dos informes frente a frente, con solo las comprobaciones cuyo resultado difiere. */
export function ComparePage() {
  const { snapshot, suite } = useLive();
  const [params, setParams] = useSearchParams();
  const a = params.get("a") ?? "";
  const b = params.get("b") ?? "";
  const [draft, setDraft] = useState({ a, b });
  const [comparison, setComparison] = useState<Comparison | null>(null);
  const [complaint, setComplaint] = useState<string | null>(null);
  const aId = useId();
  const bId = useId();
  const reports = (snapshot?.reports ?? []).filter((entry) => !entry.complaint);

  useEffect(() => {
    document.title = "Comparar informes · Suite de conformidad";
  }, []);

  useEffect(() => {
    setDraft({ a, b });
    if (!a || !b) {
      setComparison(null);
      return;
    }
    let current = true;
    setComplaint(null);
    suite
      .compare(a, b)
      .then((result) => {
        if (current) setComparison(result);
      })
      .catch((error: unknown) => {
        if (current) {
          setComparison(null);
          setComplaint(error instanceof Error ? error.message : String(error));
        }
      });
    return () => {
      current = false;
    };
  }, [suite, a, b]);

  const compare = (next: { a: string; b: string }) => {
    const search = new URLSearchParams(params);
    search.set("a", next.a);
    search.set("b", next.b);
    setParams(search);
  };

  const pick = (side: "a" | "b", id: string, label: string) => (
    <div className="field-group grow">
      <label className="field-label" htmlFor={id}>
        {label}
      </label>
      <select
        id={id}
        className="input"
        value={draft[side]}
        onChange={(event) => setDraft({ ...draft, [side]: event.target.value })}
      >
        <option value="" disabled>
          Elige un informe
        </option>
        {reports.map((entry) => (
          <option key={entry.name} value={entry.name}>
            {describe(entry)}
          </option>
        ))}
      </select>
    </div>
  );

  return (
    <>
      <header className="page-head">
        <h1>Comparar informes</h1>
        <p className="muted">Solo lo que difiere, por conjunto y en el orden del catálogo.</p>
      </header>
      <form
        className="compare-form"
        onSubmit={(event) => {
          event.preventDefault();
          compare(draft);
        }}
      >
        {pick("a", aId, "A")}
        <button
          type="button"
          className="icon-button swap"
          aria-label="Intercambiar A y B"
          title="Intercambiar A y B"
          onClick={() => setDraft({ a: draft.b, b: draft.a })}
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 14 14"
            aria-hidden="true"
            className="glyph stroke"
          >
            <path d="M2 4.5h9.5M9 2l2.5 2.5L9 7M12 9.5H2.5M5 7L2.5 9.5 5 12" />
          </svg>
        </button>
        {pick("b", bId, "B")}
        <button type="submit" className="button primary" disabled={!draft.a || !draft.b}>
          Comparar
        </button>
      </form>
      {complaint && <p className="complaint">{complaint}</p>}
      {comparison && <ComparisonTable comparison={comparison} />}
      {!comparison && !complaint && (
        <div className="placeholder">
          <p>Elige dos informes para ver en qué comprobaciones dan un resultado distinto.</p>
        </div>
      )}
    </>
  );
}

function describe(entry: ReportEntry): string {
  return `${entry.name} — ${clientName(entry.kind)} ${entry.client_version ?? ""} · ${calendarDate(entry.date)}`;
}

function SideCard({ letter, side }: { letter: string; side: Side }) {
  return (
    <div className="side-card">
      <span className="side-letter">{letter}</span>
      <div>
        <strong>
          {clientName(side.kind)} {side.client_version}
        </strong>
        <code className="muted">{side.client}</code>
      </div>
    </div>
  );
}

function ComparisonTable({ comparison }: { comparison: Comparison }) {
  const differing = comparison.rows.filter((row) => row.differ);
  const sets = [...new Set(differing.map((row) => row.set))];
  return (
    <section className="comparison" aria-label="Diferencias">
      <div className="sides">
        <SideCard letter="A" side={comparison.a} />
        <SideCard letter="B" side={comparison.b} />
        <div className="differing">
          <strong>{comparison.differing}</strong>
          <span>
            {comparison.differing === 1 ? "comprobación difiere" : "comprobaciones difieren"}
          </span>
        </div>
      </div>
      {differing.length === 0 ? (
        <div className="verdict tone-ok" role="status">
          <ResultIcon result="CONFORME" size={20} />
          <p>Los dos informes dan el mismo resultado en todas las comprobaciones.</p>
        </div>
      ) : (
        <table className="table compare-table">
          <thead>
            <tr>
              <th scope="col">Comprobación</th>
              <th scope="col">A</th>
              <th scope="col">B</th>
              <th scope="col">Fuente</th>
            </tr>
          </thead>
          {sets.map((set) => (
            <tbody key={set}>
              <tr className="group-row">
                <th scope="rowgroup" colSpan={4}>
                  {set}
                  <span className="count">{differing.filter((row) => row.set === set).length}</span>
                </th>
              </tr>
              {differing
                .filter((row) => row.set === set)
                .map((row) => (
                  <ComparisonRow key={row.id} row={row} />
                ))}
            </tbody>
          ))}
        </table>
      )}
    </section>
  );
}

function ComparisonRow({ row }: { row: Row }) {
  return (
    <tr>
      <td>
        <code>{row.id}</code>
        <span className="muted nowrap"> · cap. {row.chapter}</span>
      </td>
      <td className={`tone-${resultTone[row.a]}`}>
        <span className="result-label">
          <ResultIcon result={row.a} size={12} decorative />
          {row.a}
        </span>
      </td>
      <td className={`tone-${resultTone[row.b]}`}>
        <span className="result-label">
          <ResultIcon result={row.b} size={12} decorative />
          {row.b}
        </span>
      </td>
      <td>
        <code className="muted">{row.citation}</code>
      </td>
    </tr>
  );
}
