import type { Summary } from "../contract/Summary";
import { ResultIcon } from "../ui/icons";
import { countOf, explanationTally, RESULTS, resultTone, unexplained } from "../words";

export function SummaryBar({ summary, compact = false }: { summary: Summary; compact?: boolean }) {
  const done = summary.total - summary.pending;
  return (
    <div
      className={`summary-bar${compact ? " compact" : ""}`}
      role="img"
      aria-label={`${done} de ${summary.total} comprobaciones con resultado`}
    >
      {RESULTS.map((result) => {
        const count = result === "NO CONFORME" ? unexplained(summary) : countOf(summary, result);
        return count === 0 ? null : (
          <span
            key={result}
            className={`segment tone-${resultTone[result]}`}
            style={{ flexGrow: count }}
          />
        );
      })}
      {summary.explained > 0 && (
        <span className="segment tone-explained" style={{ flexGrow: summary.explained }} />
      )}
    </div>
  );
}

export function Counts({ summary }: { summary: Summary }) {
  return (
    <ul className="counts" aria-label="Recuento por resultado">
      {RESULTS.map((result) => {
        const count = countOf(summary, result);
        return (
          <li key={result} className={count === 0 ? "is-zero" : undefined}>
            <ResultIcon result={result} size={11} />
            <span className="count">{count}</span>
            {result === "NO CONFORME" && count > 0 && <ExplainedBadge summary={summary} />}
          </li>
        );
      })}
    </ul>
  );
}

function ExplainedBadge({ summary }: { summary: Summary }) {
  return (
    <span
      className="explained-count"
      title={`${explanationTally(summary)}. Un NO CONFORME con cualquier etiqueta está explicado`}
    >
      {summary.explained} expl.
    </span>
  );
}
