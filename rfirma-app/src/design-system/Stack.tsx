//! La pila vertical `rf-stack`, con los huecos que usa la interfaz.

import type { ComponentPropsWithRef } from "react";
import { classNames } from "./classNames";

type StackGap = "xs" | "md";

export type StackProps = ComponentPropsWithRef<"div"> & { gap?: StackGap };

export function Stack({ gap, className, ...rest }: StackProps) {
  return <div className={classNames("rf-stack", gap && `rf-gap-${gap}`, className)} {...rest} />;
}
