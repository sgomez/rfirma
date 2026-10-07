//! El botón por defecto de una pantalla o un diálogo: recibe el foco en cuanto se puede pulsar, si nadie lo ha llevado a otro sitio.

import { useEffect, useRef } from "react";
import type { PrimaryButton } from "./actionKeys";

/** El botón por defecto: el foco, en cuanto se puede pulsar y si nadie lo ha llevado a otro sitio. */
export function useDefaultButton(enabled = true) {
  const button = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (enabled) claimUnclaimedFocus(button);
  }, [enabled]);

  return button;
}

/** Da el foco al botón si se puede pulsar y el foco está en el documento o en un contenedor suyo que no se tabula. */
export function claimUnclaimedFocus(button: PrimaryButton | undefined) {
  const target = button?.current;
  if (target === null || target === undefined || target.disabled) return;
  if (focusIsUnclaimed(target)) target.focus();
}

function focusIsUnclaimed(button: HTMLElement): boolean {
  const active = document.activeElement;
  if (active === null || active === document.body) return true;
  return active instanceof HTMLElement && active.tabIndex < 0 && active.contains(button);
}
