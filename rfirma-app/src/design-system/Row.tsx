//! La fila horizontal `rf-row`, con los huecos que usa la interfaz.

import type { ComponentPropsWithRef, ElementType } from "react";
import { classNames } from "./classNames";

type RowGap = "xs" | "sm";

export type RowProps = ComponentPropsWithRef<"div"> & { gap?: RowGap; as?: ElementType };

export function Row({ gap, className, as: Tag = "div", ...rest }: RowProps) {
  return <Tag className={classNames("rf-row", gap && `rf-gap-${gap}`, className)} {...rest} />;
}
