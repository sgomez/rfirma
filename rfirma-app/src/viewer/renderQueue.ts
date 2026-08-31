import type { RenderTask } from "./pdf";

/**
 * Una sola pintada viva sobre el lienzo, siempre.
 *
 * `pdf.js` pinta de forma incremental y asíncrona: una `RenderTask` sigue
 * escribiendo sobre el `<canvas>` mucho después de haberla lanzado. Si al
 * cambiar el zoom o de página se lanza la siguiente sin cancelar la anterior,
 * las dos escriben sobre el mismo lienzo y queda una mezcla de dos escalas —o,
 * peor, la vieja termina la última y gana. Por eso hay cola.
 */
export interface RenderQueue {
  /**
   * Cancela lo que hubiera en vuelo y lanza la pintada nueva.
   *
   * La tarea se pide **después** de cancelar, no antes: entre las dos no puede
   * haber un instante con dos vivas.
   *
   * La promesa resuelve cuando la pintada termina **o cuando se cancela** —una
   * cancelación es lo que se pidió, no un fallo—. Un fallo de verdad sí sale.
   */
  run(start: () => RenderTask): Promise<void>;
  /** Cancela lo que haya en vuelo. Es lo que llama el desmontaje del visor. */
  cancel(): void;
}

/**
 * Lo que `pdf.js` lanza al cancelar una pintada.
 *
 * Se reconoce por el nombre y no con `instanceof RenderingCancelledException`
 * para no arrastrar `pdfjs-dist` hasta aquí: este módulo es la lógica que las
 * pruebas de la grada A ejercitan con dobles, y cargar la librería entera en
 * `jsdom` para comparar una clase sería pagar mucho por un `===`.
 */
function isCancellation(error: unknown): boolean {
  return error instanceof Error && error.name === "RenderingCancelledException";
}

/** Crea una cola. Hay una por lienzo, y por tanto una por visor. */
export function createRenderQueue(): RenderQueue {
  let inFlight: RenderTask | null = null;

  return {
    async run(start) {
      inFlight?.cancel();
      const task = start();
      inFlight = task;
      try {
        await task.promise;
      } catch (error) {
        if (!isCancellation(error)) throw error;
      } finally {
        // Solo si sigue siendo la nuestra: si ya la sustituyó otra, borrarla
        // dejaría a la nueva sin poder cancelarse.
        if (inFlight === task) inFlight = null;
      }
    },
    cancel() {
      inFlight?.cancel();
      inFlight = null;
    },
  };
}
