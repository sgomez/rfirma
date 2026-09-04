import type { DocumentFailure } from "../viewer/source";
import type { RecentDocument } from "./recents";

/**
 * Lo que le ocurre a la ventana cuando alguien suelta ficheros encima.
 *
 * Es lo mismo que devuelve el diálogo —un documento ya abierto y
 * canonicalizado— más las dos cosas que solo pasan al soltar: que lo soltado no
 * valga, y que sean varios.
 */
export interface Drop {
  /** El documento que se ha abierto, o `null` si no se ha abierto ninguno. */
  document: RecentDocument | null;
  /** Por qué no se ha abierto ninguno. `null` cuando sí se abrió. */
  failure: DocumentFailure | null;
  /**
   * Cuántos ficheros más venían en el mismo gesto y no se han abierto.
   *
   * La aplicación firma de uno en uno, así que se abre el primero que sea un
   * PDF **y se dice** (ID-70): callarse los demás dejaría a la persona sin
   * saber cuál de los cinco que soltó tiene delante.
   */
  ignored: number;
}

/**
 * Por dónde entra un arrastre.
 *
 * Es un puerto **suscribible** y no una llamada, porque el arrastre no se pide:
 * ocurre. Y es un puerto propio, con su doble, por una razón muy concreta del
 * ID-67: en Tauri v2 el WebView trae desactivados los eventos de arrastre de
 * HTML a favor del evento nativo, así que un `onDrop` en el JSX **no se
 * dispararía nunca** y parecería un fallo del frontal. Detrás de este puerto
 * está ese evento nativo; delante, una ventana que se prueba entera sin
 * backend.
 *
 * Quien decide qué se abre de lo soltado es el backend, no la ventana: lo que
 * se suelta son rutas del anfitrión y ninguna cruza (ADR-0011).
 */
export interface DocumentDrops {
  /** Escucha los arrastres. Devuelve con qué dejar de escuchar. */
  subscribe(listener: (drop: Drop) => void): () => void;
  /**
   * Lo que ya venía cuando se abrió la ventana: el documento con el que se
   * invocó a la aplicación desde fuera, `rfirma documento.pdf` (ID-157).
   *
   * Es una llamada y no una suscripción porque el documento se conoce **antes**
   * de que la ventana exista: emitirlo al arrancar sería emitirlo al vacío. Y
   * llega por este puerto, y no por uno propio, porque desemboca en lo mismo —
   * la ventana completa en el estado en que la deja arrastrar un PDF (ID-159)—,
   * y dos caminos parecidos son dos estados que se separan.
   *
   * `null` cuando la aplicación se abrió sin documento, que es lo normal. Se
   * consume: preguntar dos veces no lo trae dos veces.
   */
  pending(): Promise<Drop | null>;
}

/**
 * Un puerto de arrastre con el que además se puede soltar. Es el doble.
 *
 * Se construye con lo que traía la invocación, si traía algo: así los dos
 * caminos —la invocación y el arrastre— se prueban por la ventana, que es donde
 * tienen que acabar en el mismo sitio.
 */
export interface FakeDocumentDrops extends DocumentDrops {
  /** Suelta esto en la ventana, como si alguien lo hubiera arrastrado. */
  drop(drop: Drop): void;
}

/**
 * El arrastre sin Tauri: entrega a quien escuche lo que se le suelte.
 *
 * Es el doble de las pruebas, y con él los cuatro casos del arrastre se
 * comprueban como comportamiento observable en la ventana (TD-17). Quien habla
 * con el evento nativo es `tauriDocumentDrops`.
 */
export function inMemoryDocumentDrops(pending: Drop | null = null): FakeDocumentDrops {
  const listeners = new Set<(drop: Drop) => void>();
  let waiting = pending;
  return {
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    pending: async () => {
      const invoked = waiting;
      waiting = null;
      return invoked;
    },
    drop: (drop) => {
      for (const listener of listeners) listener(drop);
    },
  };
}

/** Un arrastre que no ocurre nunca. Es el relleno de una composición sin él. */
export function noDocumentDrops(): DocumentDrops {
  return { subscribe: () => () => {}, pending: async () => null };
}
