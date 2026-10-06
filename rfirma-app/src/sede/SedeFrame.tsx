//! El cuerpo y el pie de cada momento (`SedeBody`), con su Intro y su Escape, y los hooks que comparten los momentos: el cierre solo del desenlace y la cuenta atrás de consentir.

import { type ReactNode, useEffect, useRef, useState } from "react";
import { type PrimaryButton, useActionKeys } from "../design-system/actionKeys";
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
  primary,
  onEscape,
}: {
  children: ReactNode;
  footer: ReactNode;
  steadyFooter?: boolean;
  flush?: boolean;
  primary?: PrimaryButton;
  onEscape?: () => void;
}) {
  useActionKeys({ primary, onSecondary: onEscape });

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
