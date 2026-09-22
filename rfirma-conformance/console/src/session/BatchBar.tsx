import { useEffect } from "react";
import type { QuestionView } from "../contract/QuestionView";
import { useLive } from "../suite/live";
import { Elapsed } from "../ui/Elapsed";
import { ActivityIcon, SkipIcon, StopIcon } from "../ui/icons";
import { notify } from "../ui/notify";
import { useShortcuts } from "../ui/shortcuts";
import type { Batch } from "./progress";

/** La tanda en curso, única dueña de saltar, detener y responder. */
export function BatchBar({ batch, question }: { batch: Batch; question: QuestionView | null }) {
  const { suite, complain } = useLive();
  const skip = () => void suite.skip().catch(complain("No se pudo saltar"));
  const stop = () => void suite.stop().catch(complain("No se pudo detener"));
  const answer = (given: string | null) =>
    void suite.answer(given).catch(complain("No se pudo responder"));
  const briefing = question?.kind === "briefing";
  const tranche = question?.kind === "tranche";
  const trancheCheck = tranche ? question.check : null;
  const tranchePrompt = tranche ? question.prompt : null;

  useEffect(() => {
    if (trancheCheck && tranchePrompt) notify(tranchePrompt);
  }, [trancheCheck, tranchePrompt]);

  useShortcuts({
    x: skip,
    X: stop,
    s: question ? () => answer("s") : undefined,
    n: question && !briefing && !tranche ? () => answer("n") : undefined,
    Escape: question ? () => answer(null) : undefined,
  });

  return (
    <section className="batch-bar" aria-label="En curso">
      <div className="batch-line">
        <ActivityIcon activity={question ? "asking" : "running"} />
        <div className="batch-what">
          <span className="batch-set">{batch.set ?? "…"}</span>
          <span className="batch-sep" aria-hidden="true">
            /
          </span>
          <code className="batch-check">{batch.current}</code>
        </div>
        <div className="batch-progress">
          <progress
            className="meter"
            value={batch.done}
            max={Math.max(1, batch.total)}
            aria-label={`${batch.done} de ${batch.total} terminadas`}
          />
          <span className="batch-count">
            {batch.done}/{batch.total}
          </span>
        </div>
        <span className="batch-time" title="Tiempo desde que empezó">
          <Elapsed since={batch.startedAt} />
        </span>
        <div className="batch-actions">
          <button type="button" className="button small" onClick={skip} aria-keyshortcuts="x">
            <SkipIcon /> Saltar esta <kbd>x</kbd>
          </button>
          <button
            type="button"
            className="button small danger"
            onClick={stop}
            aria-keyshortcuts="Shift+X"
          >
            <StopIcon /> Detener <kbd>⇧X</kbd>
          </button>
        </div>
      </div>
      {question && (
        <div className="question" role="alertdialog" aria-labelledby="question-title">
          <div className="question-text">
            <span className="question-kind" id="question-title">
              {tranche ? "Cambio de tramo" : briefing ? "Antes de empezar" : "Te preguntamos"}
            </span>
            <p className="question-prompt">{question.prompt.replace(/\s*\[s\/n\]\s*$/, "")}</p>
          </div>
          <div className="question-actions">
            {tranche ? (
              <>
                <button type="button" className="button primary" onClick={() => answer("s")}>
                  Estoy <kbd>s</kbd>
                </button>
                <button type="button" className="button ghost" onClick={() => answer(null)}>
                  Detener aquí: lo demás queda pendiente <kbd>Esc</kbd>
                </button>
              </>
            ) : briefing ? (
              <>
                <button type="button" className="button primary" onClick={() => answer("s")}>
                  Empezar <kbd>s</kbd>
                </button>
                <button type="button" className="button ghost" onClick={() => answer(null)}>
                  Saltar: queda pendiente <kbd>Esc</kbd>
                </button>
              </>
            ) : (
              <>
                <button type="button" className="button primary" onClick={() => answer("s")}>
                  Sí <kbd>s</kbd>
                </button>
                <button type="button" className="button" onClick={() => answer("n")}>
                  No <kbd>n</kbd>
                </button>
                <button type="button" className="button ghost" onClick={() => answer(null)}>
                  Descartar: queda pendiente <kbd>Esc</kbd>
                </button>
              </>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
