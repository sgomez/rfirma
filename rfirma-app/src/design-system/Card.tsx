//! La tarjeta `rf-card`, llana o elevada.

import type { ComponentPropsWithRef } from "react";
import { classNames } from "./classNames";

export type CardProps = ComponentPropsWithRef<"div"> & { elevated?: boolean };

export function Card({ elevated, className, ...rest }: CardProps) {
  return (
    <div className={classNames("rf-card", elevated && "rf-card--elevated", className)} {...rest} />
  );
}
