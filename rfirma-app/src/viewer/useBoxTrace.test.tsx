import { fireEvent, render, screen } from "@testing-library/react";
import { useRef } from "react";
import { describe, expect, it, vi } from "vitest";
import type { PageSize, PixelRect } from "./signatureBox";
import { useBoxTrace } from "./useBoxTrace";

/**
 * **Grada A** (`vitest`, carril rápido). #190.
 *
 * Lo que se comprueba aquí es lo que la prueba del visor no ve: **el fantasma**,
 * que es lo único que se pinta mientras el trazo está en curso, y que como el
 * arrastre no pasa por el estado de React ni una sola vez.
 */

const PAGE: PageSize = { width: 400, height: 600 };
const MIN: PageSize = { width: 120, height: 34 };

/** Cuenta cuántas veces ha pintado React, que es lo que se está midiendo. */
let renders = 0;

function Probe({ onTrace = () => {} }: { onTrace?: (traced: PixelRect) => void }) {
  renders += 1;
  const sheet = useRef<HTMLDivElement>(null);
  const ghost = useRef<HTMLDivElement>(null);
  const handlers = useBoxTrace({ sheet, ghost, page: PAGE, min: MIN, onTrace });

  return (
    // biome-ignore lint/a11y/noNoninteractiveTabindex: la hoja se enfoca, como en el visor.
    <div ref={sheet} role="document" aria-label="hoja" tabIndex={0} {...handlers}>
      <div ref={ghost} data-testid="ghost" style={{ display: "none" }} />
    </div>
  );
}

function sheet() {
  return screen.getByRole("document", { name: "hoja" });
}

function ghost() {
  return screen.getByTestId("ghost");
}

describe("useBoxTrace", () => {
  it("draws the ghost while the trace is in flight, and takes it away on drop", () => {
    render(<Probe />);

    fireEvent.pointerDown(sheet(), { pointerId: 1, button: 0, clientX: 50, clientY: 60 });
    fireEvent.pointerMove(sheet(), { pointerId: 1, clientX: 250, clientY: 160 });

    expect(ghost().style.display).toBe("block");
    expect(ghost().style.left).toBe("50px");
    expect(ghost().style.top).toBe("60px");
    expect(ghost().style.width).toBe("200px");
    expect(ghost().style.height).toBe("100px");

    fireEvent.pointerUp(sheet(), { pointerId: 1, clientX: 250, clientY: 160 });

    expect(ghost().style.display).toBe("none");
  });

  // La misma razón que en `useBoxDrag`: reconciliar el árbol entero con un PDF
  // al lado, sesenta veces por segundo, se nota.
  it("never goes through React state while tracing", () => {
    render(<Probe />);
    renders = 0;

    fireEvent.pointerDown(sheet(), { pointerId: 1, button: 0, clientX: 50, clientY: 60 });
    for (let x = 60; x < 260; x += 10) {
      fireEvent.pointerMove(sheet(), { pointerId: 1, clientX: x, clientY: 100 });
    }

    expect(renders).toBe(0);
  });

  // El mínimo es una regla sobre el recuadro que queda, no sobre el gesto: el
  // fantasma sigue al cursor aunque todavía no dé el tamaño.
  it("draws the ghost the hand is drawing, without jumping to the minimum", () => {
    const onTrace = vi.fn();
    render(<Probe onTrace={onTrace} />);

    fireEvent.pointerDown(sheet(), { pointerId: 1, button: 0, clientX: 50, clientY: 60 });
    fireEvent.pointerMove(sheet(), { pointerId: 1, clientX: 90, clientY: 80 });

    expect(ghost().style.width).toBe("40px");
    expect(ghost().style.height).toBe("20px");

    // Y al soltar sí: lo que queda colocado no baja del mínimo (ID-103).
    fireEvent.pointerUp(sheet(), { pointerId: 1, clientX: 90, clientY: 80 });

    expect(onTrace).toHaveBeenCalledWith({ x: 50, y: 60, width: 120, height: 34 });
  });

  // El clic seco no coloca, pero sí enfoca la hoja: es lo que deja pasar de
  // página con `AvPág` y `RePág` (ID-113).
  it("hands the focus to the sheet when the gesture was only a click", () => {
    render(<Probe />);

    fireEvent.pointerDown(sheet(), { pointerId: 1, button: 0, clientX: 50, clientY: 60 });
    fireEvent.pointerUp(sheet(), { pointerId: 1, clientX: 50, clientY: 60 });

    expect(sheet()).toHaveFocus();
  });

  it("shows nothing while the gesture could still be a click", () => {
    const onTrace = vi.fn();
    render(<Probe onTrace={onTrace} />);

    fireEvent.pointerDown(sheet(), { pointerId: 1, button: 0, clientX: 50, clientY: 60 });
    fireEvent.pointerMove(sheet(), { pointerId: 1, clientX: 52, clientY: 62 });

    expect(ghost().style.display).toBe("none");

    fireEvent.pointerUp(sheet(), { pointerId: 1, clientX: 52, clientY: 62 });

    expect(onTrace).not.toHaveBeenCalled();
  });

  // Un trazo cancelado —el puntero se lo lleva el sistema— no coloca nada, y
  // sobre todo no deja el fantasma pintado en la hoja.
  it("leaves no ghost behind when the gesture is cancelled", () => {
    const onTrace = vi.fn();
    render(<Probe onTrace={onTrace} />);

    fireEvent.pointerDown(sheet(), { pointerId: 1, button: 0, clientX: 50, clientY: 60 });
    fireEvent.pointerMove(sheet(), { pointerId: 1, clientX: 250, clientY: 160 });
    fireEvent.pointerCancel(sheet(), { pointerId: 1 });

    expect(ghost().style.display).toBe("none");
    expect(onTrace).not.toHaveBeenCalled();
  });

  it("starts no trace with the secondary button", () => {
    const onTrace = vi.fn();
    render(<Probe onTrace={onTrace} />);

    fireEvent.pointerDown(sheet(), { pointerId: 1, button: 2, clientX: 50, clientY: 60 });
    fireEvent.pointerMove(sheet(), { pointerId: 1, clientX: 250, clientY: 160 });
    fireEvent.pointerUp(sheet(), { pointerId: 1, clientX: 250, clientY: 160 });

    expect(onTrace).not.toHaveBeenCalled();
  });
});
