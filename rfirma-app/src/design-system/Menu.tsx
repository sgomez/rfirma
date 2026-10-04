//! El menú desplegable sobre `Popover`: `role="menu"`, foco en la primera entrada, flechas, Inicio y Fin, y el cierre y la vuelta del foco del panel; cada entrada lleva el contenido que quiera.

import type { ComponentPropsWithRef, FocusEvent, KeyboardEvent } from "react";
import { classNames } from "./classNames";
import { Popover, type PopoverProps } from "./Popover";
import "./Menu.css";

export type MenuProps = Omit<PopoverProps, "role" | "initialFocus" | "tabIndex">;

export type MenuItemProps = Omit<ComponentPropsWithRef<"button">, "role" | "tabIndex">;

const ITEMS = '[role="menuitem"]:not([disabled])';

/** Un menú anclado a su botón; sus entradas son `MenuItem` y, entre grupos, un `<hr>`. */
export function Menu({ onKeyDown, onFocus, className, ...rest }: MenuProps) {
  const focusFirstItem = (event: FocusEvent<HTMLDivElement>) => {
    onFocus?.(event);
    if (event.target === event.currentTarget) itemsOf(event.currentTarget).at(0)?.focus();
  };

  const moveFocus = (event: KeyboardEvent<HTMLDivElement>) => {
    onKeyDown?.(event);
    const items = itemsOf(event.currentTarget);
    const target = indexToFocus(
      event.key,
      items.indexOf(event.target as HTMLElement),
      items.length,
    );
    if (target === null) return;
    event.preventDefault();
    items[target]?.focus();
  };

  return (
    <Popover
      role="menu"
      tabIndex={-1}
      initialFocus="panel"
      onFocus={focusFirstItem}
      onKeyDown={moveFocus}
      className={classNames("rf-menu", className)}
      {...rest}
    />
  );
}

/** Una entrada del menú, fuera del orden de tabulación como pide `role="menu"`. */
export function MenuItem({ className, type = "button", ...rest }: MenuItemProps) {
  return (
    <button
      type={type}
      role="menuitem"
      tabIndex={-1}
      className={classNames("rf-menu__item", className)}
      {...rest}
    />
  );
}

function itemsOf(menu: HTMLElement): HTMLElement[] {
  return Array.from(menu.querySelectorAll<HTMLElement>(ITEMS));
}

function indexToFocus(key: string, at: number, count: number): number | null {
  if (count === 0) return null;
  switch (key) {
    case "ArrowDown":
      return (at + 1) % count;
    case "ArrowUp":
      return at <= 0 ? count - 1 : at - 1;
    case "Home":
      return 0;
    case "End":
      return count - 1;
    default:
      return null;
  }
}
