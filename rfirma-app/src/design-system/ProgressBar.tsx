//! La barra de progreso `rf-progress`, con su `role="progressbar"`; el nombre accesible lo pone quien la usa.

import type { ComponentPropsWithRef } from "react";
import { classNames } from "./classNames";
import "./ProgressBar.css";

type ProgressBarVariant = "framed";

export type ProgressBarProps = Omit<ComponentPropsWithRef<"div">, "children"> & {
  value: number;
  min?: number;
  max?: number;
  variant?: ProgressBarVariant;
};

export function ProgressBar({
  value,
  min = 0,
  max = 100,
  variant,
  className,
  ...rest
}: ProgressBarProps) {
  const fraction = (value - min) / (max - min);
  return (
    <div
      role="progressbar"
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      className={classNames("rf-progress", variant && `rf-progress--${variant}`, className)}
      {...rest}
    >
      <span className="rf-progress__fill" style={{ width: `${fraction * 100}%` }} />
    </div>
  );
}
