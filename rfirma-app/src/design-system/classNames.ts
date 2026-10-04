//! Une las clases que hay y descarta las que no: lo que hace `clsx` en los primitivos, sin la dependencia.

export function classNames(...parts: (string | false | null | undefined)[]): string {
  return parts.filter(Boolean).join(" ");
}
