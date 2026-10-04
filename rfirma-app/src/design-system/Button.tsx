//! El botón del sistema de diseño: `rf-btn` con las variantes que usa la interfaz, `type="button"` por defecto.

import type { ComponentPropsWithRef } from "react";
import { classNames } from "./classNames";

type ButtonVariant = "primary" | "secondary" | "ghost";

export type ButtonProps = ComponentPropsWithRef<"button"> & { variant?: ButtonVariant };

export function Button({ variant, className, type = "button", ...rest }: ButtonProps) {
  return (
    <button
      type={type}
      className={classNames("rf-btn", variant && `rf-btn--${variant}`, className)}
      {...rest}
    />
  );
}
