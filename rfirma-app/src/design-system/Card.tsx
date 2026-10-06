//! La tarjeta `rf-card`, llana o elevada.

import type { ComponentPropsWithRef, ElementType } from "react";
import { classNames } from "./classNames";

export type CardProps = ComponentPropsWithRef<"div"> & { elevated?: boolean; as?: ElementType };

export function Card({ elevated, className, as: Tag = "div", ...rest }: CardProps) {
  return (
    <Tag className={classNames("rf-card", elevated && "rf-card--elevated", className)} {...rest} />
  );
}
