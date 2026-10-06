//! Las pestañas del sistema de diseño: `role="tablist"` con sus `role="tab"`, `aria-selected` y tabulación itinerante con flechas; el contenido de cada pestaña es de quien las usa.

import type { ComponentPropsWithRef, KeyboardEvent } from "react";
import { classNames } from "./classNames";
import "./Tabs.css";

export type TabsProps = Omit<ComponentPropsWithRef<"div">, "role">;

export type TabProps = Omit<
  ComponentPropsWithRef<"button">,
  "role" | "tabIndex" | "aria-selected"
> & {
  selected: boolean;
};

const TABS = '[role="tab"]:not([disabled])';

/** La tira de pestañas; sus hijas son `Tab`, que la persona activa por interfaz con un clic o Intro. */
export function Tabs({ onKeyDown, className, ...rest }: TabsProps) {
  const moveFocus = (event: KeyboardEvent<HTMLDivElement>) => {
    onKeyDown?.(event);
    const tabs = Array.from(event.currentTarget.querySelectorAll<HTMLElement>(TABS));
    const target = indexToFocus(event.key, tabs.indexOf(event.target as HTMLElement), tabs.length);
    if (target === null) return;
    event.preventDefault();
    tabs[target]?.focus();
  };

  return (
    <div
      role="tablist"
      onKeyDown={moveFocus}
      className={classNames("rf-tabs", className)}
      {...rest}
    />
  );
}

/** Una pestaña: solo la seleccionada entra en el orden de tabulación. */
export function Tab({ selected, className, type = "button", ...rest }: TabProps) {
  return (
    <button
      type={type}
      role="tab"
      aria-selected={selected}
      tabIndex={selected ? 0 : -1}
      className={classNames("rf-tabs__tab", className)}
      {...rest}
    />
  );
}

function indexToFocus(key: string, at: number, count: number): number | null {
  if (count === 0 || at < 0) return null;
  switch (key) {
    case "ArrowRight":
      return (at + 1) % count;
    case "ArrowLeft":
      return at === 0 ? count - 1 : at - 1;
    case "Home":
      return 0;
    case "End":
      return count - 1;
    default:
      return null;
  }
}
