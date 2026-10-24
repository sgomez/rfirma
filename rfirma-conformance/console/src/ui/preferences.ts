import { useCallback, useState } from "react";

function read<T>(key: string, fallback: T, accepts: (value: unknown) => boolean): T {
  try {
    const raw = window.localStorage.getItem(key);
    if (raw === null) return fallback;
    const value: unknown = JSON.parse(raw);
    return accepts(value) ? (value as T) : fallback;
  } catch {
    return fallback;
  }
}

function write(key: string, value: unknown) {
  try {
    window.localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // sin almacenamiento, la preferencia vale hasta recargar
  }
}

/** Un estado que sobrevive a la recarga en el navegador; sin almacenamiento, es un `useState`. */
export function useStoredState<T>(
  key: string,
  initial: T,
  accepts: (value: unknown) => boolean,
): [T, (next: T | ((was: T) => T)) => void] {
  const [value, setValue] = useState<T>(() => read(`conformance.${key}`, initial, accepts));
  const set = useCallback(
    (next: T | ((was: T) => T)) => {
      setValue((was) => {
        const resolved = typeof next === "function" ? (next as (was: T) => T)(was) : next;
        write(`conformance.${key}`, resolved);
        return resolved;
      });
    },
    [key],
  );
  return [value, set];
}

export const isBoolean = (value: unknown): value is boolean => typeof value === "boolean";

export const isNumber = (value: unknown): value is number =>
  typeof value === "number" && Number.isFinite(value);

export const isListOf =
  (allowed: readonly string[]) =>
  (value: unknown): boolean =>
    Array.isArray(value) && value.every((each) => allowed.includes(each as string));
