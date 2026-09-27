import { useEffect } from "react";

type ShortcutKeys = Pick<
  KeyboardEvent,
  "key" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey" | "repeat"
>;

/** Si la tecla es Ctrl+O, o Cmd+O en macOS. */
export function isOpenShortcut(event: ShortcutKeys, userAgent: string): boolean {
  const macOS = /mac os x|macintosh/i.test(userAgent);
  const command = macOS ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
  return (
    command && !event.altKey && !event.shiftKey && !event.repeat && event.key.toLowerCase() === "o"
  );
}

/** Ctrl+O (Cmd+O en macOS) abre un PDF mientras `enabled` lo permita. */
export function useOpenShortcut(open: () => void, enabled: boolean): void {
  useEffect(() => {
    if (!enabled) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (!isOpenShortcut(event, navigator.userAgent)) return;
      event.preventDefault();
      open();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, enabled]);
}
