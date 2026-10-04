//! El campo de formulario `rf-field`: etiqueta, control y ayuda apilados.

import type { ComponentPropsWithRef } from "react";
import { classNames } from "./classNames";

export type FieldProps = ComponentPropsWithRef<"div">;

export function Field({ className, ...rest }: FieldProps) {
  return <div className={classNames("rf-field", className)} {...rest} />;
}
