//! El atajo común de teclado: Intro pulsa la acción primaria y Escape la secundaria, salvo que un control ya atienda la tecla o haya un diálogo delante; entre pantallas anidadas, la interior primero.

import { type RefObject, useEffect, useRef, useState } from "react";

/** La referencia al botón primario de una pantalla o un diálogo. */
export type PrimaryButton = RefObject<HTMLButtonElement | null>;

/** La primaria y la secundaria de una pantalla o un diálogo, cada una opcional. */
export type ActionKeys = {
  primary?: PrimaryButton;
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
  primary: PrimaryButton | undefined,
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

type Registration = { mountOrder: number; actions: RefObject<ActionKeys> };

const registrations: Registration[] = [];
let mounted = 0;

// Un padre se monta antes que sus hijos: el orden de montaje anida como el árbol.
function nextMountOrder(): number {
  mounted += 1;
  return mounted;
}

function answerInnermostFirst(event: KeyboardEvent) {
  if (layers.length > 0) return;
  const innermost = [...registrations]
    .sort((a, b) => b.mountOrder - a.mountOrder)
    .find(({ actions }) => declaresKey(event, actions.current));
  if (innermost === undefined) return;
  const { primary, onSecondary } = innermost.actions.current;
  if (event.key === "Enter") pressPrimaryOnEnter(event, primary);
  else pressSecondaryOnEscape(event, onSecondary);
}

function declaresKey(event: KeyboardEvent, { primary, onSecondary }: ActionKeys): boolean {
  return (
    (event.key === "Enter" && primary !== undefined) ||
    (event.key === "Escape" && onSecondary !== undefined)
  );
}

/** El atajo de una pantalla completa o de una confirmación dentro de ella: atiende el documento mientras no hay un diálogo delante, la más interior antes. */
export function useActionKeys(actions: ActionKeys) {
  const latest = useRef(actions);
  latest.current = actions;
  const [mountOrder] = useState(nextMountOrder);

  useEffect(() => {
    const registration = { mountOrder, actions: latest };
    registrations.push(registration);
    if (registrations.length === 1) document.addEventListener("keydown", answerInnermostFirst);
    return () => {
      registrations.splice(registrations.indexOf(registration), 1);
      if (registrations.length === 0) document.removeEventListener("keydown", answerInnermostFirst);
    };
  }, [mountOrder]);
}
