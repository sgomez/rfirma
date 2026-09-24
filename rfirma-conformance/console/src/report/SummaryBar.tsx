import type { Summary } from "../contract/Summary";
import { ResultIcon } from "../ui/icons";
import { countOf, DEPRECATED_LABEL, RESULTS, resultTone } from "../words";

export function SummaryBar({ summary, compact = false }: { summary: Summary; compact?: boolean }) {
  const done = summary.total - summary.pending;
  return (
    <div
      className={`summary-bar${compact ? " compact" : ""}`}
      role="img"
      aria-label={`${done} de ${summary.total} comprobaciones con resultado`}
    >
      {RESULTS.map((result) => {
        const count = countOf(summary, result);
        return count === 0 ? null : (
          <span
            key={result}
            className={`segment tone-${resultTone[result]}`}
            style={{ flexGrow: count }}
          />
        );
      })}
      {summary.deprecated > 0 && (
        <span className="segment tone-deprecated" style={{ flexGrow: summary.deprecated }} />
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
          </li>
        );
      })}
      {summary.deprecated > 0 && (
        <li className="deprecated-count" title={DEPRECATED_LABEL}>
          <span className="count">{summary.deprecated} deprecados</span>
        </li>
      )}
    </ul>
  );
}
