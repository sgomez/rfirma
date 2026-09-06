import { type ReactNode, useEffect, useRef, useState } from "react";
import { OUTCOME_CLOSE_MS, UNREACHABLE_AFTER_MS, WAITING_GRACE_MS } from "./errand";

/**
 * El cuerpo y el pie, que son iguales en los cinco momentos.
 *
 * El pie de los momentos de firma y de salida mide **56 px clavados**, con
 * `height` fijo, para que aparecer y desaparecer «Cancelar» no mueva nada de
 * sitio.
 */
export function SedeBody({
  children,
  footer,
  steadyFooter = false,
}: {
  children: ReactNode;
  footer: ReactNode;
  steadyFooter?: boolean;
}) {
  return (
    <>
      <div className="sede-window__body">{children}</div>
      <footer
        className={`rf-row rf-gap-xs sede-window__footer${
          steadyFooter ? " sede-window__footer--steady" : ""
        }`}
      >
        {footer}
      </footer>
    </>
  );
}

/**
 * Los dos relojes de la espera: el retardo antes de pintar y el único umbral
 * que hay. Vive aquí y no en `SedeWaiting` para que la pantalla siga siendo
 * una función de su entrada.
 */
export function useWaitingClock(): "hidden" | "connecting" | "unreachable" {
  const [elapsed, setElapsed] = useState<"hidden" | "connecting" | "unreachable">("hidden");

  useEffect(() => {
    const grace = setTimeout(() => setElapsed("connecting"), WAITING_GRACE_MS);
    const threshold = setTimeout(() => setElapsed("unreachable"), UNREACHABLE_AFTER_MS);
    return () => {
      clearTimeout(grace);
      clearTimeout(threshold);
    };
  }, []);

  return elapsed;
}

/**
 * El cierre solo del desenlace, a los quince segundos (ID-274).
 *
 * El cierre viaja en una referencia y no en las dependencias: quien nos monta
 * pasa una función anónima nueva en cada pintada, y con ella en la lista la
 * cuenta se reiniciaría sin parar y la ventana no se cerraría jamás.
 */
export function useOutcomeClock(onClose: () => void) {
  const latest = useRef(onClose);
  latest.current = onClose;

  useEffect(() => {
    const timer = setTimeout(() => latest.current(), OUTCOME_CLOSE_MS);
    return () => clearTimeout(timer);
  }, []);
}
