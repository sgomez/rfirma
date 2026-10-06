//! La pila vertical `rf-stack`, con los huecos que usa la interfaz.

import type { ComponentPropsWithRef, ElementType } from "react";
import { classNames } from "./classNames";

type StackGap = "xs" | "md";

export type StackProps = ComponentPropsWithRef<"div"> & { gap?: StackGap; as?: ElementType };

export function Stack({ gap, className, as: Tag = "div", ...rest }: StackProps) {
  return <Tag className={classNames("rf-stack", gap && `rf-gap-${gap}`, className)} {...rest} />;
}
