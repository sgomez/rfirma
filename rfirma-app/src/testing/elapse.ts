//! `elapse`, el auxiliar de las pruebas con relojes falsos: deja pasar el tiempo y que React repinte.

import { act } from "@testing-library/react";
import { vi } from "vitest";

/** Deja pasar el tiempo con los relojes falsos, y deja que React repinte. */
export async function elapse(ms: number) {
  await act(async () => {
    vi.advanceTimersByTime(ms);
  });
}
