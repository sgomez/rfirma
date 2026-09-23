import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { expect, vi } from "vitest";
import type { PdfDocument, PdfPage, RenderTask, Viewport } from "../pdf";
import type { Placement } from "../signatureBox";

/**
 * Lo que comparten las pruebas de `DocumentViewer` (grada A, `vitest`,
 * sub-issue #58): el doble de `pdf.js` y los atajos de consulta del DOM.
 *
 * `pdf.js` no cabe en `jsdom` —no hay contexto `2d`—, así que el documento
 * entra por el puerto de `pdf.ts` y aquí se enchufa un doble que registra lo
 * que le piden: qué páginas, a qué escala y cuáles se cancelaron.
 */

export const A4 = { width: 595, height: 842 };

/** Un viewport con la transformación de `pdf.js` sin rotación: escala y voltea. */
export function viewportAt(scale: number): Viewport {
  return {
    width: A4.width * scale,
    height: A4.height * scale,
    convertToPdfPoint: (x, y) => [x / scale, A4.height - y / scale],
    convertToViewportPoint: (x, y) => [x * scale, (A4.height - y) * scale],
  };
}

export interface Recorder {
  document: PdfDocument;
  /** Las pintadas lanzadas, en orden. */
  renders: Array<{ page: number; scale: number; cancelled: boolean; finish: () => void }>;
}

/** Un documento cuyas pintadas no acaban solas: las termina la prueba. */
export function recordingDocument(pageCount = 3): Recorder {
  const renders: Recorder["renders"] = [];

  const pageOf = (number: number): PdfPage => ({
    number,
    rotate: 0,
    view: [0, 0, 595, 842],
    getViewport: ({ scale }) => viewportAt(scale),
    render: ({ viewport }) => {
      let settle: () => void = () => {};
      let fail: (error: unknown) => void = () => {};
      const entry = {
        page: number,
        scale: viewport.width / A4.width,
        cancelled: false,
        finish: () => settle(),
      };
      renders.push(entry);
      const task: RenderTask = {
        promise: new Promise<void>((resolve, reject) => {
          settle = resolve;
          fail = reject;
        }),
        cancel: () => {
          entry.cancelled = true;
          const cancellation = new Error("Rendering cancelled");
          cancellation.name = "RenderingCancelledException";
          fail(cancellation);
        },
      };
      return task;
    },
  });

  return {
    renders,
    document: { pageCount, getPage: (number) => Promise.resolve(pageOf(number)) },
  };
}

export const noop = () => {};

/**
 * Un recuadro ya colocado. Desde el ID-114 el visor **no lo crea**: quien lo
 * quiera en pantalla lo entrega, igual que hace la fila que lo recuerda.
 */
export const seated: Placement = {
  rect: { x0: 50, y0: 60, x1: 250, y1: 140 },
  pages: { only: [1] },
};

export function box() {
  return screen.getByRole("application", { name: "Recuadro de la firma visible" });
}

export function sheet() {
  return screen.getByRole("document", { name: "Hoja del documento" });
}

/** Se teclea la página en la barra y se espera a que la pintada llegue. */
export async function goToPage(wanted: number, renders: Recorder["renders"]) {
  const painted = renders.length;
  fireEvent.change(screen.getByLabelText("Número de página"), {
    target: { value: String(wanted) },
  });
  await waitFor(() => expect(renders.length).toBeGreaterThan(painted));
}

/** La parte visible del visor, la que se mide para ajustar. */
export function surfaceOf(container: HTMLElement): HTMLElement {
  return container.querySelector(".viewer__scroll") as HTMLElement;
}

/**
 * `jsdom` no trae `ResizeObserver` ni mide nada, así que la parte visible se
 * dimensiona a mano y el redimensionado se dispara desde la prueba.
 */
export function stubResizeObserver() {
  let notify: () => void = () => {};
  class Stub {
    constructor(callback: () => void) {
      notify = callback;
    }
    observe() {}
    disconnect() {}
    unobserve() {}
  }
  vi.stubGlobal("ResizeObserver", Stub);
  return {
    resizeTo(element: HTMLElement, width: number, height: number) {
      Object.defineProperty(element, "clientWidth", { value: width, configurable: true });
      Object.defineProperty(element, "clientHeight", { value: height, configurable: true });
      act(() => notify());
    },
  };
}
