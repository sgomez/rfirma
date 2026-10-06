//! El atajo común de teclado: Intro pulsa la acción primaria y Escape la secundaria, salvo que un control ya atienda la tecla o haya un diálogo delante.

import { type RefObject, useEffect, useRef } from "react";

/** La primaria y la secundaria de una pantalla o un diálogo, cada una opcional. */
export type ActionKeys = {
  primary?: RefObject<HTMLButtonElement | null>;
  onSecondary?: () => void;
};

const USES_ENTER = [
  "button",
  "a[href]",
  "input",
  "select",
  "textarea",
  '[contenteditable=""]',
  '[contenteditable="true"]',
  ...[
    "button",
    "link",
    "combobox",
    "listbox",
    "option",
    "menu",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "tab",
    "switch",
    "checkbox",
    "radio",
    "textbox",
    "spinbutton",
  ].map((role) => `[role="${role}"]`),
].join(", ");

const layers: HTMLElement[] = [];

/** Apila una capa modal y devuelve cómo sacarla de la pila. */
export function enterLayer(layer: HTMLElement): () => void {
  layers.push(layer);
  return () => {
    layers.splice(layers.indexOf(layer), 1);
  };
}

/** Si la capa es la de arriba de la pila, la única que atiende el teclado. */
export function isTopLayer(layer: HTMLElement): boolean {
  return layers.at(-1) === layer;
}

/** Intro pulsa la primaria si nadie ha atendido la tecla, el foco no está en un control que la use y la primaria está activa. */
export function pressPrimaryOnEnter(
  event: KeyboardEvent,
  primary: RefObject<HTMLButtonElement | null> | undefined,
): boolean {
  if (event.key !== "Enter" || event.defaultPrevented || targetUsesEnter(event)) return false;
  const button = primary?.current;
  if (button === null || button === undefined || button.disabled) return false;
  event.preventDefault();
  button.click();
  return true;
}

/** Escape pulsa la secundaria si nadie ha atendido la tecla. */
function pressSecondaryOnEscape(
  event: KeyboardEvent,
  onSecondary: (() => void) | undefined,
): boolean {
  if (event.key !== "Escape" || event.defaultPrevented || onSecondary === undefined) return false;
  event.preventDefault();
  onSecondary();
  return true;
}

function targetUsesEnter(event: KeyboardEvent): boolean {
  return event.target instanceof Element && event.target.closest(USES_ENTER) !== null;
}

/** El atajo de una pantalla completa: atiende el documento mientras no hay un diálogo delante. */
export function useActionKeys(actions: ActionKeys) {
  const latest = useRef(actions);
  latest.current = actions;

  useEffect(() => {
    const listener = (event: KeyboardEvent) => {
      if (layers.length > 0) return;
      const { primary, onSecondary } = latest.current;
      if (!pressPrimaryOnEnter(event, primary)) pressSecondaryOnEscape(event, onSecondary);
    };
    document.addEventListener("keydown", listener);
    return () => document.removeEventListener("keydown", listener);
  }, []);
}
