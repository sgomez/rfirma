//! El cuerpo y el pie de cada momento (`SedeBody`), con el atajo común de Intro y Escape, y los hooks que comparten los momentos: el cierre solo del desenlace y la cuenta atrás de consentir.

import { type ReactNode, type RefObject, useEffect, useRef, useState } from "react";
import { answerActionKey } from "../design-system/actionKeys";
import { Row } from "../design-system/Row";
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
  flush = false,
  onEscape,
  primary,
}: {
  children: ReactNode;
  footer: ReactNode;
  steadyFooter?: boolean;
  flush?: boolean;
  onEscape?: () => void;
  primary?: RefObject<HTMLButtonElement | null>;
}) {
  useActionKeys(primary, onEscape);

  return (
    <>
      <div className={`sede-window__body${flush ? " sede-window__body--flush" : ""}`}>
        {children}
      </div>
      <Row
        as="footer"
        gap="xs"
        className={`sede-window__footer${steadyFooter ? " sede-window__footer--steady" : ""}`}
      >
        {footer}
      </Row>
    </>
  );
}

/**
 * El cierre solo del desenlace, a los quince segundos.
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

/** Intro pulsa la primaria del momento y Escape su salida, salvo que un control ya lo haya atendido. */
function useActionKeys(
  primary: RefObject<HTMLButtonElement | null> | undefined,
  onEscape: (() => void) | undefined,
) {
  const latest = useRef({ primary, secondary: onEscape });
  latest.current = { primary, secondary: onEscape };

  useEffect(() => {
    const listener = (event: KeyboardEvent) => {
      answerActionKey(event, latest.current);
    };
    document.addEventListener("keydown", listener);
    return () => document.removeEventListener("keydown", listener);
  }, []);
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
