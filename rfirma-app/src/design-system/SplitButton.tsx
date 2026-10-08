//! El botón partido: una acción principal y una flecha que abre un menú; compuesto de `Button` y `Menu`, con la semántica del menú resuelta aquí.

import {
  Children,
  type MouseEvent,
  type ReactNode,
  useCallback,
  useId,
  useRef,
  useState,
} from "react";
import { Button, type ButtonProps } from "./Button";
import { classNames } from "./classNames";
import { Icon } from "./icons";
import { Menu, type MenuProps } from "./Menu";
import "./SplitButton.css";

export type SplitButtonProps = {
  /** El contenido de la acción principal. */
  children: ReactNode;
  onAction: () => void;
  /** El nombre accesible de la flecha; también su `title`. */
  menuLabel: string;
  /** Las entradas del menú (`MenuItem`); sin ellas solo se pinta la acción. */
  items?: ReactNode;
  title?: string;
  className?: string;
  menuClassName?: MenuProps["className"];
  variant?: ButtonProps["variant"];
};

/** La acción principal y, a su lado, la flecha del menú; elegir una entrada lo cierra. */
export function SplitButton({
  children,
  onAction,
  menuLabel,
  items,
  title,
  className,
  menuClassName,
  variant = "secondary",
}: SplitButtonProps) {
  const [open, setOpen] = useState(false);
  const container = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const menuId = useId();
  const close = useCallback(() => setOpen(false), []);
  const hasMenu = Children.count(items) > 0;

  const closeOnPick = (event: MouseEvent<HTMLDivElement>) => {
    if ((event.target as HTMLElement).closest('[role="menuitem"]')) close();
  };

  return (
    <div className={classNames("rf-split", className)} ref={container}>
      <Button variant={variant} className="rf-split__action" title={title} onClick={onAction}>
        {children}
      </Button>
      {hasMenu && (
        <>
          <span className="rf-split__divider" aria-hidden="true" />
          <Button
            ref={trigger}
            variant={variant}
            className="rf-split__arrow"
            title={menuLabel}
            aria-label={menuLabel}
            aria-haspopup="menu"
            aria-expanded={open}
            aria-controls={open ? menuId : undefined}
            onClick={() => setOpen((was) => !was)}
          >
            <Icon name="dropdown" size={14} strokeWidth={2} />
          </Button>
          <Menu
            open={open}
            onClose={close}
            anchorRef={container}
            returnFocusRef={trigger}
            id={menuId}
            aria-label={menuLabel}
            className={menuClassName}
            onClick={closeOnPick}
          >
            {items}
          </Menu>
        </>
      )}
    </div>
  );
}
