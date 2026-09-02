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

/** El tamaño de la parte visible del visor, en píxeles CSS. */
export interface ObservedSize {
  width: number;
  height: number;
}

/**
 * Avisa del tamaño de la parte visible cada vez que cambia, y devuelve cómo
 * dejar de mirar.
 *
 * Vive junto a la cola porque lo que dispara es **otra pintada**: quien eligió
 * «ajustar al ancho» ha dicho *cómo* quiere mirar, no *cuánto* quiere ampliar,
 * así que estirar la ventana recalcula la escala y repinta (ID-117). El
 * redimensionado llega a ráfagas, y cada aviso entra por la misma cola que
 * cancela la anterior: sin eso, arrastrar el borde de la ventana deja media
 * docena de `RenderTask` escribiendo sobre el mismo lienzo.
 *
 * Sustituye a un «ajustar a la ventana» de un solo cálculo, que quedaba
 * desajustado en cuanto la ventana cambiaba de tamaño.
 *
 * Mide con `clientWidth`/`clientHeight` y no con el `contentRect` de la
 * entrada: son los mismos píxeles CSS que usa el resto del visor, así que el
 * observador es el disparador y no una segunda fuente de medidas.
 */
export function observeSize(
  element: HTMLElement,
  onSize: (size: ObservedSize) => void,
): () => void {
  // Ningún WebView soportado se queda sin `ResizeObserver`; `jsdom` sí, y sin
  // esta guardia se caería aquí cualquier prueba que monte el visor.
  if (typeof ResizeObserver === "undefined") return () => {};
  const observer = new ResizeObserver(() => {
    onSize({ width: element.clientWidth, height: element.clientHeight });
  });
  observer.observe(element);
  return () => observer.disconnect();
}
