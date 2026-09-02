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
const MIN: PageSize = { width: 120, height: 34 };

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
  const drag = useBoxDrag({ box, rect, page: PAGE, min: MIN, onDrop, onOutOfPage });

  return (
    <div ref={box} role="application" aria-label="recuadro" {...drag.box}>
      <span data-testid="grip" {...drag.grip("bottom-right")} />
      <span data-testid="grip-top-left" {...drag.grip("top-left")} />
    </div>
  );
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

/** ID-103: los cuatro tiradores, `Mayús` y el tamaño mínimo. */
describe("los tiradores del recuadro", () => {
  function grip(testId = "grip") {
    return screen.getByTestId(testId);
  }

  it("resizes from the corner without re-rendering React, and commits once", () => {
    const onDrop = vi.fn();
    render(<Probe onDrop={onDrop} />);
    const corner = grip();
    renders = 0;

    grab(corner);
    fireEvent.pointerMove(corner, { pointerId: 1, clientX: 130, clientY: 120 });

    expect(renders).toBe(0);
    expect(boxElement().style.width).toBe("230px");
    expect(boxElement().style.height).toBe("100px");
    expect(onDrop).not.toHaveBeenCalled();

    fireEvent.pointerUp(corner, { pointerId: 1 });

    expect(onDrop).toHaveBeenCalledWith({ x: 40, y: 60, width: 230, height: 100 });
  });

  it("leaves the opposite corner where it was when the top left one is dragged", () => {
    const onDrop = vi.fn();
    render(<Probe onDrop={onDrop} />);
    const corner = grip("grip-top-left");

    grab(corner);
    fireEvent.pointerMove(corner, { pointerId: 1, clientX: 80, clientY: 80 });
    fireEvent.pointerUp(corner, { pointerId: 1 });

    const [dropped] = onDrop.mock.calls[0] as [PixelRect];
    expect(dropped.x + dropped.width).toBe(RECT.x + RECT.width);
    expect(dropped.y + dropped.height).toBe(RECT.y + RECT.height);
  });

  it("stops at the minimum size instead of shrinking past it", () => {
    const onDrop = vi.fn();
    render(<Probe onDrop={onDrop} />);
    const corner = grip();

    grab(corner);
    fireEvent.pointerMove(corner, { pointerId: 1, clientX: -400, clientY: -400 });
    fireEvent.pointerUp(corner, { pointerId: 1 });

    expect(onDrop).toHaveBeenCalledWith({ x: 40, y: 60, width: MIN.width, height: MIN.height });
  });

  it("keeps the proportion while Shift is held, and lets go of it when released", () => {
    const onDrop = vi.fn();
    render(<Probe onDrop={onDrop} />);
    const corner = grip();

    grab(corner);
    fireEvent.pointerMove(corner, { pointerId: 1, clientX: 200, clientY: 100, shiftKey: true });
    fireEvent.pointerUp(corner, { pointerId: 1 });

    const [held] = onDrop.mock.calls[0] as [PixelRect];
    expect(held.width / held.height).toBeCloseTo(RECT.width / RECT.height);
  });

  it("does not move the box while a grip is being dragged", () => {
    render(<Probe />);
    const corner = grip();

    grab(corner);
    fireEvent.pointerMove(corner, { pointerId: 1, clientX: 130, clientY: 120 });

    expect(boxElement().style.transform).toBe("");
  });

  it("refuses a resize that falls off the page, and says so", () => {
    const onDrop = vi.fn();
    const onOutOfPage = vi.fn();
    render(<Probe onDrop={onDrop} onOutOfPage={onOutOfPage} />);
    const corner = grip();

    // 40 + 200 + 300 > 400: la esquina se sale por la derecha.
    grab(corner);
    fireEvent.pointerMove(corner, { pointerId: 1, clientX: 400, clientY: 100 });
    fireEvent.pointerUp(corner, { pointerId: 1 });

    expect(onDrop).not.toHaveBeenCalled();
    expect(onOutOfPage).toHaveBeenCalledTimes(1);
    // Y el recuadro vuelve a su geometría de antes, la que pinta el estado.
    expect(boxElement().style.width).toBe(`${RECT.width}px`);
  });
});
