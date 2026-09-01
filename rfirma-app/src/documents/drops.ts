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
}

/** Un puerto de arrastre con el que además se puede soltar. Es el doble. */
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
export function inMemoryDocumentDrops(): FakeDocumentDrops {
  const listeners = new Set<(drop: Drop) => void>();
  return {
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    drop: (drop) => {
      for (const listener of listeners) listener(drop);
    },
  };
}

/** Un arrastre que no ocurre nunca. Es el relleno de una composición sin él. */
export function noDocumentDrops(): DocumentDrops {
  return { subscribe: () => () => {} };
}
