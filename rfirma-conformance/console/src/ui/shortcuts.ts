import { useEffect, useRef } from "react";

export type Keymap = Record<string, (() => void) | undefined>;

function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target.isContentEditable ||
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement
  );
}

/** Atajos de una tecla, mudos al escribir, con un modificador pulsado o bajo un diálogo modal. */
export function useShortcuts(keymap: Keymap) {
  const current = useRef(keymap);
  current.current = keymap;

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey) return;
      if (isTyping(event.target) || document.querySelector("dialog[open]")) return;
      const action = current.current[event.key];
      if (!action) return;
      event.preventDefault();
      action();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}

export function moveAmongChecks(step: 1 | -1) {
  const rows = [...document.querySelectorAll<HTMLElement>("[data-check-row]")];
  if (rows.length === 0) return;
  const at = rows.indexOf(document.activeElement as HTMLElement);
  const next = at === -1 ? (step === 1 ? 0 : rows.length - 1) : at + step;
  const row = rows[Math.min(rows.length - 1, Math.max(0, next))];
  row?.focus();
  row?.scrollIntoView({ block: "nearest" });
}

export function focusedCheck(): string | null {
  const row = document.activeElement?.closest<HTMLElement>("[data-check]");
  return row?.dataset.check ?? null;
}
