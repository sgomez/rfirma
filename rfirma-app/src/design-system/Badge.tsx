//! La insignia `rf-badge`, neutra o primaria.

import type { ComponentPropsWithRef } from "react";
import { classNames } from "./classNames";

type BadgeVariant = "primary";

export type BadgeProps = ComponentPropsWithRef<"span"> & { variant?: BadgeVariant };

export function Badge({ variant, className, ...rest }: BadgeProps) {
  return (
    <span
      className={classNames("rf-badge", variant && `rf-badge--${variant}`, className)}
      {...rest}
    />
  );
}
