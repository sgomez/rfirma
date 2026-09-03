import type { PdfDocument } from "../viewer/pdf";
import type { DocumentFailure } from "../viewer/source";
import type { SigningOrder } from "./flow";

/**
 * El sello que se ve dentro del recuadro **antes** de firmar (ID-107).
 *
 * La regla del ADR-0006, tal y como la cerró la ficha 7: **o es el sello de
 * verdad, o no hay recuadro**. No se maqueta nada aquí ni en ninguna parte del
 * frontal — lo que se pinta es un PDF que ya lleva el sello estampado, salido
 * de un **ciclo trifásico en seco** con un `PK1` inventado, cuyos bytes
 * visibles están medidos idénticos a los del firmado de verdad
 * (`docs/research/prefirma-en-seco-pdfjs.md`). El compositor es el mismo, así
 * que no hay una segunda opinión que pueda discrepar.
 *
 * Y la segunda regla, del mismo signo: **la vista previa no es una puerta**
 * (ID-111). Si el sello no se puede componer se dice y se firma igual; sobre si
 * se puede firmar manda el botón de firmar, que no mira nada de este módulo.
 */

/** Lo que sale de componer el sello: el PDF en seco, o el fallo que lo impidió. */
export type ComposedStamp =
  | { ok: true; pdf: PdfDocument }
  | { ok: false; failure: DocumentFailure };

/**
 * Quien compone el sello: un ciclo en seco entero, que se tira.
 *
 * Es un puerto por lo mismo que lo son el origen del PDF y el compositor del
 * texto: debajo hay una orden de Tauri que llama al puente, y los cuatro
 * estados de esta vista previa son de la interfaz y no dependen de que haya un
 * token puesto (TD-32).
 *
 * Devuelve el PDF **ya abierto** y no los bytes: quien sabe abrirlos es
 * `pdf.js`, que vive del otro lado de la frontera, y pasar por aquí un
 * `Uint8Array` obligaría a cada doble de prueba a fabricar un PDF de verdad.
 */
export interface StampComposer {
  compose(order: SigningOrder): Promise<ComposedStamp>;
}

/**
 * Lo que la ventana le pide a la vista previa, que **no siempre es un sello**.
 *
 * Los dos casos que no lo son se nombran, en vez de resumirse en un `null`,
 * porque el panel los cuenta distinto: sin certificado el bloque entero de
 * firma visible está apagado (ID-108), y sin colocar lo que falta es el gesto
 * sobre la hoja.
 */
export type StampRequest =
  | { kind: "noCertificate" }
  | { kind: "unplaced" }
  | { kind: "ready"; order: SigningOrder };

/**
 * En qué estado está la vista previa.
 *
 * Los cuatro de la ficha —sin certificado, sin colocar, congelada durante el
 * gesto y compuesta— más los tres que son de camino: componiendo, a la espera
 * de «Ver cómo queda» en un documento grande, y el fallo del ID-111.
 */
export type StampPreview =
  | { kind: "noCertificate" }
  | { kind: "unplaced" }
  /** El gesto está en curso: la vista anterior se congela y se atenúa (ID-109). */
  | { kind: "frozen" }
  /** Documento grande: el recálculo se pide con «Ver cómo queda» (ID-109). */
  | { kind: "onDemand" }
  | { kind: "composing" }
  | { kind: "composed" }
  /** No se ha podido componer. **No apaga el botón de firmar** (ID-111). */
  | { kind: "failed"; failure: DocumentFailure };

/**
 * Por encima de este tamaño el recálculo deja de ser automático y se pide con
 * «Ver cómo queda» (ID-109).
 *
 * El umbral no está medido punto a punto: lo que está medido son los dos
 * extremos —0,15 s en un PDF de 2,4 MB y 1,9 s con 507 MB de RSS en un
 * escaneado de 37 MB—, y ocho megas caen entre ellos, más cerca del que va
 * solo. Se elige el tamaño y no el tiempo del ciclo anterior porque el tamaño
 * se sabe **antes** de pagar el primer ciclo, y el tiempo sólo después.
 */
export const ON_DEMAND_BYTES = 8 * 1024 * 1024;

/**
 * Si la vista previa se recalcula sola al soltar el recuadro.
 *
 * Con el tamaño desconocido se recalcula: el caso corriente es un documento
 * pequeño, y esperar un botón que nadie ha pedido es peor que un ciclo de más.
 */
export function composesOnRelease(sizeBytes: number | null): boolean {
  return sizeBytes === null || sizeBytes < ON_DEMAND_BYTES;
}

/**
 * Un compositor que **no compone nada**, y lo dice.
 *
 * Sirve para montar la ventana en una prueba sin backend, y falla diciendo la
 * verdad en vez de dejar el recuadro vacío: enseñar una caja vacía sería
 * exactamente la aproximación que el ID-107 prohíbe.
 */
export function unavailableStampComposer(): StampComposer {
  return {
    compose: async () => ({
      ok: false,
      failure: {
        situation: "documentUnreadable",
        detail: "esta composicion no compone el sello",
      },
    }),
  };
}
