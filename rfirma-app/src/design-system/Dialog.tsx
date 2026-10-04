//! El diálogo modal `rf-dialog` sobre su velo: foco dentro, tabulador que no sale y, si se puede cerrar, Escape.

import { type ComponentPropsWithoutRef, useEffect, useRef } from "react";
import { classNames } from "./classNames";

type DialogRole = "dialog" | "alertdialog";

export type DialogProps = Omit<
  ComponentPropsWithoutRef<"div">,
  "role" | "aria-label" | "aria-modal" | "tabIndex"
> & {
  /** El nombre accesible del diálogo. */
  label: string;
  /** Lo que hace Escape; sin él, el diálogo no se cierra con el teclado. */
  onClose?: () => void;
  role?: DialogRole;
  /** La clase del velo, para quien lo coloca distinto. */
  scrimClassName?: string;
};

const FOCUSABLE = 'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';

const open: HTMLElement[] = [];

function focusableWithin(dialog: HTMLElement): HTMLElement[] {
  return [...dialog.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
    (element) => !element.hasAttribute("disabled") && element.tabIndex !== -1,
  );
}

function keepTabInside(event: KeyboardEvent, dialog: HTMLElement) {
  const focusable = focusableWithin(dialog);
  const first = focusable.at(0);
  const last = focusable.at(-1);
  if (first === undefined || last === undefined) {
    event.preventDefault();
    dialog.focus();
    return;
  }
  const active = document.activeElement;
  if (!dialog.contains(active)) {
    event.preventDefault();
    (event.shiftKey ? last : first).focus();
  } else if (event.shiftKey && (active === first || active === dialog)) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && active === last) {
    event.preventDefault();
    first.focus();
  }
}

/** El diálogo modal sobre su velo; el más reciente es el único que atiende el teclado. */
export function Dialog({
  label,
  onClose,
  role = "dialog",
  className,
  scrimClassName,
  ...rest
}: DialogProps) {
  const dialog = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const element = dialog.current;
    if (element === null) return;
    const previous = document.activeElement;
    open.push(element);
    element.focus();
    return () => {
      open.splice(open.indexOf(element), 1);
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const element = dialog.current;
      if (element === null || open.at(-1) !== element || event.defaultPrevented) return;
      if (event.key === "Tab") {
        keepTabInside(event, element);
      } else if (event.key === "Escape" && onClose !== undefined) {
        event.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    <div className={classNames("rf-scrim", scrimClassName)}>
      {/* biome-ignore lint/a11y/useAriaPropsSupportedByRole: el rol es siempre dialog o alertdialog, que admiten aria-modal. */}
      <div
        {...rest}
        className={classNames("rf-dialog", className)}
        role={role}
        aria-modal="true"
        aria-label={label}
        tabIndex={-1}
        ref={dialog}
      />
    </div>
  );
}
