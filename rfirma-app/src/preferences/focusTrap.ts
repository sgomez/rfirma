import type { KeyboardEvent } from "react";

/** Lo que puede recibir el foco dentro de un modal. */
const FOCUSABLE = 'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';

/**
 * El tabulador da la vuelta dentro del modal en vez de salirse a la pantalla
 * que queda detrás. Solo se aplica a los dos modales que se ponen delante
 * —confirmar el borrado, la contraseña del `.p12`—: la pantalla en sí ya no
 * atrapa el foco, así que el menú de la cabecera sigue alcanzable con el
 * teclado mientras Preferencias está delante.
 */
export function trapFocus(modal: HTMLElement | null, event: KeyboardEvent<HTMLDivElement>) {
  if (modal === null) return;
  const focusable = [...modal.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
    (element) => !element.hasAttribute("disabled") && element.tabIndex !== -1,
  );
  const first = focusable.at(0);
  const last = focusable.at(-1);
  if (first === undefined || last === undefined) return;
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

/** El tabulador da la vuelta dentro del modal al que está enganchado. */
export function trapTabWithinCurrentTarget(event: KeyboardEvent<HTMLDivElement>) {
  if (event.key === "Tab") trapFocus(event.currentTarget, event);
}
