//! El atajo común de teclado: Intro pulsa la acción primaria y Escape la secundaria, y el foco por defecto en la primaria.

import { type RefObject, useEffect, useRef } from "react";

/** La primaria y la secundaria de una pantalla o de un diálogo, cada una opcional. */
export type Actions = {
  primary?: RefObject<HTMLButtonElement | null>;
  secondary?: () => void;
};

const USES_ENTER =
  'button, a[href], input, select, textarea, [contenteditable="true"], [role="button"], [role="link"], [role="combobox"], [role="listbox"], [role="option"], [role="menuitem"], [role="switch"], [role="tab"]';

/** Atiende Intro o Escape con las acciones dadas y dice si lo ha hecho. */
export function answerActionKey(event: KeyboardEvent, { primary, secondary }: Actions): boolean {
  if (event.defaultPrevented) return false;
  if (event.key === "Escape" && secondary !== undefined) {
    event.preventDefault();
    secondary();
    return true;
  }
  if (event.key !== "Enter" || targetUsesEnter(event)) return false;
  const button = primary?.current;
  if (button == null || !isPressable(button)) return false;
  event.preventDefault();
  button.click();
  return true;
}

function targetUsesEnter(event: KeyboardEvent): boolean {
  return event.target instanceof Element && event.target.closest(USES_ENTER) !== null;
}

function isPressable(button: HTMLButtonElement): boolean {
  return button.isConnected && !button.disabled && button.getAttribute("aria-disabled") !== "true";
}

/** El botón por defecto: el foco, en cuanto se puede pulsar y si nadie lo ha llevado a otro sitio. */
export function useDefaultButton(enabled = true) {
  const button = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const element = button.current;
    if (enabled && element !== null && focusIsUnclaimed(element)) element.focus();
  }, [enabled]);

  return button;
}

function focusIsUnclaimed(button: HTMLElement): boolean {
  const active = document.activeElement;
  return active === null || active === document.body || active.contains(button);
}
