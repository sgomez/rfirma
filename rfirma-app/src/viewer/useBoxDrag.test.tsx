import { fireEvent, render, screen } from "@testing-library/react";
import { useRef } from "react";
import { describe, expect, it, vi } from "vitest";
import type { PageSize, PixelRect } from "./signatureBox";
import { useBoxDrag } from "./useBoxDrag";

/**
 * **Grada A** (`vitest`, carril rápido).
 *
 * Las dos cosas que el sub-issue #58 pide del arrastre: que **no pase por el
 * estado de React** en cada `pointermove` —se nota a 60 fps— y que soltar el
 * recuadro fuera de la página no se acepte en silencio (ID-22).
 */

const PAGE: PageSize = { width: 400, height: 600 };
const RECT: PixelRect = { x: 40, y: 60, width: 200, height: 80 };

/** Cuenta cuántas veces ha pintado React, que es lo que se está midiendo. */
let renders = 0;

function Probe({
  onDrop = () => {},
  onOutOfPage = () => {},
  rect = RECT,
}: {
  onDrop?: (moved: PixelRect) => void;
  onOutOfPage?: () => void;
  rect?: PixelRect;
}) {
  renders += 1;
  const box = useRef<HTMLDivElement>(null);
  const drag = useBoxDrag({ box, rect, page: PAGE, onDrop, onOutOfPage });

  return <div ref={box} role="application" aria-label="recuadro" {...drag} />;
}

function boxElement() {
  return screen.getByRole("application", { name: "recuadro" });
}

function grab(element: HTMLElement, x = 100, y = 100) {
  fireEvent.pointerDown(element, { pointerId: 1, button: 0, clientX: x, clientY: y });
}

describe("el arrastre del recuadro", () => {
  it("does not re-render React while the pointer moves", () => {
    render(<Probe />);
    const element = boxElement();
    renders = 0;

    grab(element);
    for (let step = 1; step <= 10; step += 1) {
      fireEvent.pointerMove(element, { pointerId: 1, clientX: 100 + step, clientY: 100 + step });
    }

    // Diez `pointermove` y ni una pintada: el gesto va por la `ref`.
    expect(renders).toBe(0);
    expect(element.style.transform).toBe("translate(10px, 10px)");
  });

  it("commits the move once, on pointerup", () => {
    const onDrop = vi.fn();
    render(<Probe onDrop={onDrop} />);
    const element = boxElement();

    grab(element);
    fireEvent.pointerMove(element, { pointerId: 1, clientX: 130, clientY: 150 });
    expect(onDrop).not.toHaveBeenCalled();

    fireEvent.pointerUp(element, { pointerId: 1 });

    expect(onDrop).toHaveBeenCalledTimes(1);
    expect(onDrop).toHaveBeenCalledWith({ x: 70, y: 110, width: 200, height: 80 });
    // El `transform` del gesto se retira: la posición nueva la pinta el estado.
    expect(element.style.transform).toBe("");
  });

  it("refuses a drop that falls off the page, and says so", () => {
    const onDrop = vi.fn();
    const onOutOfPage = vi.fn();
    render(<Probe onDrop={onDrop} onOutOfPage={onOutOfPage} />);
    const element = boxElement();

    // Hacia la derecha hasta salirse: 40 + 300 + 200 > 400.
    grab(element);
    fireEvent.pointerMove(element, { pointerId: 1, clientX: 400, clientY: 100 });
    fireEvent.pointerUp(element, { pointerId: 1 });

    expect(onOutOfPage).toHaveBeenCalledTimes(1);
    expect(onDrop).not.toHaveBeenCalled();
    expect(element.style.transform).toBe("");
  });

  it("ignores moves that never started with a pointerdown", () => {
    const onDrop = vi.fn();
    render(<Probe onDrop={onDrop} />);
    const element = boxElement();

    fireEvent.pointerMove(element, { pointerId: 1, clientX: 300, clientY: 300 });
    fireEvent.pointerUp(element, { pointerId: 1 });

    expect(element.style.transform).toBe("");
    expect(onDrop).not.toHaveBeenCalled();
  });

  it("gives up the gesture when the pointer is cancelled", () => {
    const onDrop = vi.fn();
    render(<Probe onDrop={onDrop} />);
    const element = boxElement();

    grab(element);
    fireEvent.pointerMove(element, { pointerId: 1, clientX: 130, clientY: 130 });
    fireEvent.pointerCancel(element, { pointerId: 1 });

    expect(element.style.transform).toBe("");
    expect(onDrop).not.toHaveBeenCalled();
  });

  it("only follows the pointer that started the gesture", () => {
    render(<Probe />);
    const element = boxElement();

    grab(element);
    fireEvent.pointerMove(element, { pointerId: 7, clientX: 300, clientY: 300 });

    expect(element.style.transform).toBe("");
  });

  it("does not start a gesture with the secondary button", () => {
    const onDrop = vi.fn();
    render(<Probe onDrop={onDrop} />);
    const element = boxElement();

    fireEvent.pointerDown(element, { pointerId: 1, button: 2, clientX: 100, clientY: 100 });
    fireEvent.pointerMove(element, { pointerId: 1, clientX: 130, clientY: 130 });
    fireEvent.pointerUp(element, { pointerId: 1 });

    expect(onDrop).not.toHaveBeenCalled();
  });
});
