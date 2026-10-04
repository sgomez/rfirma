//! La fila horizontal `rf-row`, con los huecos que usa la interfaz.

import type { ComponentPropsWithRef } from "react";
import { classNames } from "./classNames";

type RowGap = "xs" | "sm";

export type RowProps = ComponentPropsWithRef<"div"> & { gap?: RowGap };

export function Row({ gap, className, ...rest }: RowProps) {
  return <div className={classNames("rf-row", gap && `rf-gap-${gap}`, className)} {...rest} />;
}
