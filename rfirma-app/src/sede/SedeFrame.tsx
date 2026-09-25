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
  onEscape,
}: {
  children: ReactNode;
  footer: ReactNode;
  steadyFooter?: boolean;
  onEscape?: () => void;
}) {
  useEscapeKey(onEscape);

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

/** Escape pulsa el botón de cancelar o cerrar del momento, salvo que un control ya lo haya atendido. */
function useEscapeKey(onEscape: (() => void) | undefined) {
  const latest = useRef(onEscape);
  latest.current = onEscape;

  useEffect(() => {
    const listener = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      latest.current?.();
    };
    document.addEventListener("keydown", listener);
    return () => document.removeEventListener("keydown", listener);
  }, []);
}

/** El botón por defecto del momento: el foco, en cuanto se puede pulsar y si nadie lo ha llevado a otro sitio. */
export function useDefaultButton(enabled = true) {
  const button = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (enabled && focusIsUnclaimed()) button.current?.focus();
  }, [enabled]);

  return button;
}

function focusIsUnclaimed(): boolean {
  return document.activeElement === null || document.activeElement === document.body;
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
