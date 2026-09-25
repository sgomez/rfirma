import { useEffect } from "react";
import type { CallView } from "../contract/CallView";
import { useLive } from "../suite/live";
import { Elapsed } from "../ui/Elapsed";
import { ActivityIcon, SkipIcon, StopIcon } from "../ui/icons";
import { notify } from "../ui/notify";
import { useShortcuts } from "../ui/shortcuts";
import type { Batch } from "./progress";

/** La tanda en curso, única dueña de saltar, detener y dar paso. */
export function BatchBar({ batch, call }: { batch: Batch; call: CallView | null }) {
  const { suite, complain } = useLive();
  const skip = () => void suite.skip().catch(complain("No se pudo saltar"));
  const stop = () => void suite.stop().catch(complain("No se pudo detener"));
  const answer = (given: string | null) =>
    void suite.answer(given).catch(complain("No se pudo responder"));
  const tranche = call?.kind === "tranche";
  const trancheCheck = tranche ? call.check : null;
  const tranchePrompt = tranche ? call.prompt : null;

  useEffect(() => {
    if (trancheCheck && tranchePrompt) notify(tranchePrompt);
  }, [trancheCheck, tranchePrompt]);

  useShortcuts({
    x: skip,
    X: stop,
    s: call ? () => answer("s") : undefined,
    Escape: call ? () => answer(null) : undefined,
  });

  return (
    <section className="batch-bar" aria-label="En curso">
      <div className="batch-line">
        <ActivityIcon activity={call ? "waiting" : "running"} />
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
      {call && (
        <div className="call" role="alertdialog" aria-labelledby="call-title">
          <div className="call-text">
            <span className="call-kind" id="call-title">
              {tranche ? "Te necesitamos delante" : "Antes de empezar"}
            </span>
            <p className="call-prompt">{call.prompt}</p>
          </div>
          <div className="call-actions">
            {tranche ? (
              <>
                <button type="button" className="button primary" onClick={() => answer("s")}>
                  Estoy aquí <kbd>s</kbd>
                </button>
                <button type="button" className="button ghost" onClick={() => answer(null)}>
                  Detener aquí: lo demás queda pendiente <kbd>Esc</kbd>
                </button>
              </>
            ) : (
              <>
                <button type="button" className="button primary" onClick={() => answer("s")}>
                  Empezar <kbd>s</kbd>
                </button>
                <button type="button" className="button ghost" onClick={() => answer(null)}>
                  Saltar: queda pendiente <kbd>Esc</kbd>
                </button>
              </>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
