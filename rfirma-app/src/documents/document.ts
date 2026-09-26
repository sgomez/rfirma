import type { Placement } from "../viewer/signatureBox";

/** Si el documento ya lleva firmas; es el `Badge` de `memory::recents` en el backend. */
export type Badge = "Signed" | "Unsigned";

/**
 * **El documento que la aplicación tiene delante**, que no es la fila que se
 * guarda en la bandeja (ID-287).
 *
 * Es el hermano en la ventana de `app::in_hand::DocumentInHand`: lo que hace
 * falta para pintar y firmar el documento en curso, más la única cosa que hay
 * que saber para no dejar rastro de él cuando no se debe. Hasta aquí los dos
 * conceptos eran el mismo tipo, y el único camino para tener un documento
 * delante era escribir su fila; eso deja de valer en cuanto quien manda el
 * documento es una sede, porque **de ese no se guarda nada** (ID-286).
 *
 * Lo que no está aquí es lo que solo tiene sentido **en la lista**: cuándo se
 * usó por última vez y si la ruta responde ahora mismo. Un documento en curso
 * no se ordena ni se pinta en la bandeja: se lee y se firma.
 */
export interface DocumentInHand {
  /**
   * El identificador **opaco** que acuñó el backend al abrir el documento
   * (ID-62). Es lo que la ventana manda de vuelta en cada orden; no es una
   * ruta y de él no se puede reconstruir ninguna (ADR-0011).
   */
  id: string;
  /** El nombre del fichero, para la cabecera y el panel. */
  name: string;
  /** Si ya venía firmado, hasta donde se sabe sin abrirlo. */
  badge: Badge;
  /** El `mtime`, en segundos desde la época; `null` si no se pudo leer. */
  modified: number | null;
  /**
   * Dónde va el recuadro **en este documento**, o `null` si nadie lo ha
   * colocado (ID-74).
   *
   * De un documento que se recuerda llega repuesto desde su fila; de uno que
   * no, es siempre lo que la ventana tenga puesto ahora mismo, y al cerrarlo
   * se pierde.
   */
  placement: Placement | null;
  /**
   * **Si de este documento queda rastro.**
   *
   * Es el interruptor del ID-286, y viaja con el documento y no con la fila
   * porque se decide por dónde entró: lo que abre el diálogo o el arrastre se
   * recuerda; lo que mandará una sede, no. Con él en `false` no hay fila en
   * Recientes, no se guarda la colocación del recuadro y firmarlo tampoco
   * anota nada.
   */
  remembered: boolean;
}
