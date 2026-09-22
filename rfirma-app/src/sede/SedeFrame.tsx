import { type ReactNode, useEffect, useRef, useState } from "react";
import { CONSENT_COUNTDOWN_SECONDS, OUTCOME_CLOSE_MS } from "./errand";

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
 * El cierre solo del desenlace, a los quince segundos (ID-274).
 *
 * El cierre viaja en una referencia y no en las dependencias: quien nos monta
 * pasa una función anónima nueva en cada pintada, y con ella en la lista la
 * cuenta se reiniciaría sin parar y la ventana no se cerraría jamás.
 */
export function useOutcomeClock(onClose: () => void, enabled = true) {
  const latest = useRef(onClose);
  latest.current = onClose;

  useEffect(() => {
    if (!enabled) return;
    const timer = setTimeout(() => latest.current(), OUTCOME_CLOSE_MS);
    return () => clearTimeout(timer);
  }, [enabled]);
}

/** Los segundos que le faltan al botón de consentir para activarse; cero sin cuenta atrás. */
export function useConsentCountdown(enabled: boolean): number {
  const [remaining, setRemaining] = useState(enabled ? CONSENT_COUNTDOWN_SECONDS : 0);

  useEffect(() => {
    if (remaining === 0) return;
    const timer = setTimeout(() => setRemaining((seconds) => seconds - 1), 1000);
    return () => clearTimeout(timer);
  }, [remaining]);

  return remaining;
}
