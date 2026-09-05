import type { Placement } from "../viewer/signatureBox";
import type { Badge, DocumentInHand, ShownBadge } from "./document";

/**
 * Cuántos se recuerdan. Es el mismo `memory::recents::CAPACITY` del backend:
 * la bandeja no tiene buscador, y si algún día hace falta uno este límite
 * estaba mal.
 */
export const CAPACITY = 10;

/**
 * **La fila que se guarda**, con lo que hace falta para pintarla **sin abrir el
 * documento**.
 *
 * No es el documento que se tiene delante —eso es
 * [`DocumentInHand`](./document.ts)— sino lo que se persiste de él: se
 * deduplica por ruta canónica, se ordena y sobrevive al reinicio. De un
 * documento que no se recuerda no existe ninguna (ID-286, ID-287).
 *
 * Los cinco primeros campos son los de `memory::recents::RecentDocument`, que
 * es quien los persiste; `available` no se guarda nunca porque es un hecho
 * sobre el disco de ahora mismo, y quien lo sabe es el backend al listar.
 */
export interface RecentDocument {
  /**
   * El identificador **opaco** que acuñó el backend al abrir el documento, y
   * que es lo que identifica la fila (ID-62).
   *
   * No es una ruta y de él no se puede reconstruir ninguna: bajo el sandbox la
   * aplicación no conoce la ruta original de un documento —el portal solo se la
   * da a un llamante `is_host`, que un flatpak nunca es—, así que guardar aquí
   * una ruta era guardar una mentira. Quien sabe a qué documento del portal
   * corresponde es el registro del backend, y solo él.
   */
  id: string;
  /** El nombre del fichero, cacheado. */
  name: string;
  /** La insignia cacheada: se conoce abriendo el documento, y por eso se cachea. */
  badge: Badge;
  /** El `mtime` cacheado, en segundos desde la época; `null` si no se pudo leer. */
  modified: number | null;
  /** Cuándo se usó por última vez, en segundos desde la época. */
  lastUsed: number;
  /** Si la ruta responde ahora mismo. */
  available: boolean;
  /**
   * Dónde cayó el recuadro **en este documento**, o `null` si nadie lo colocó
   * todavía (ID-74).
   *
   * Va en la fila y no en un ajuste global porque reponer sobre un documento
   * nuevo una posición elegida para otro es lo que rechaza el ID-22. El
   * **tamaño** sí es global, y quien junta las dos mitades es el backend: aquí
   * llega el rectángulo entero.
   */
  placement: Placement | null;
}

/**
 * La insignia que se pinta: la cacheada, o `No disponible` si la ruta ya no
 * responde.
 *
 * `No disponible` es distinto de los otros dos valores: no describe el
 * documento sino la ruta, y por eso no se guarda y se recalcula al listar.
 */
export function shownBadge(document: RecentDocument): ShownBadge {
  return document.available ? document.badge : "Unavailable";
}

/**
 * La fila de la bandeja, **puesta delante**.
 *
 * Es el camino de vuelta del ID-287: elegir una fila no es firmar la fila,
 * es tomar en la mano el documento que hay detrás. Lo que se cae por el camino
 * —cuándo se usó, si responde— es de la lista y no del trabajo; lo que se
 * añade es que de este sí queda rastro, porque fila tiene.
 */
export function taken(row: RecentDocument): DocumentInHand {
  return {
    id: row.id,
    name: row.name,
    badge: row.badge,
    modified: row.modified,
    placement: row.placement,
    remembered: true,
  };
}

/**
 * Mete un documento al frente de la lista, desalojando por el final.
 *
 * La identidad es el identificador opaco, así que **volver a elegir una fila
 * de la bandeja la rescata** al frente en vez de duplicarla. Reabrir el mismo
 * fichero **por el diálogo** es otra cosa: el backend acuña un identificador
 * nuevo por cada concesión del portal (ID-62), así que el mismo
 * `contrato.pdf` abierto tres veces son tres filas. Es lo buscado —el
 * identificador nombra la concesión, no el fichero— y el precio es que esas
 * tres desalojan a las demás.
 *
 * El orden de la lista es el que manda para el desalojo; `lastUsed` es dato
 * para pintar la fila.
 */
export function record(
  recents: readonly RecentDocument[],
  document: RecentDocument,
): RecentDocument[] {
  const others = recents.filter((entry) => entry.id !== document.id);
  return [document, ...others].slice(0, CAPACITY);
}

/**
 * La fila con su recuadro cambiado. La lista se recorre entera porque `record`
 * puede haberla reordenado.
 */
export function place(
  recents: readonly RecentDocument[],
  id: string,
  placement: Placement | null,
): RecentDocument[] {
  return recents.map((entry) => (entry.id === id ? { ...entry, placement } : entry));
}

/**
 * Quita una fila de la lista.
 *
 * Es lo que ofrece una fila `No disponible` al pulsarla, y lo único que la
 * saca: **nadie la purga por su cuenta**. Un PDF en un USB desmontado o en un
 * disco de red caído no está borrado, y la fila revive cuando la ruta
 * reaparece.
 */
export function forget(recents: readonly RecentDocument[], id: string): RecentDocument[] {
  return recents.filter((entry) => entry.id !== id);
}

/**
 * De dónde salen los recientes y a dónde vuelven.
 *
 * Es un puerto y no una llamada a Tauri por la misma razón que
 * `LanguagePreference`: quien los guarda es el backend (`memory::State`), y la
 * ventana no tiene por qué saber si eso es un fichero en el disco o un objeto
 * en memoria. La implementación de verdad es `tauriRecents`, y quien elige es
 * `main.tsx`.
 */
export interface RecentsStore {
  /** La lista entera, la más reciente primero, con `available` ya resuelto. */
  list(): Promise<RecentDocument[]>;
  /**
   * Registra un documento recién abierto —o vuelve a registrarlo con otro
   * recuadro— y **devuelve la fila resultante**.
   *
   * Devuelve la fila y no nada porque es donde la ventana recupera lo que el
   * backend ya sabía de ese documento: su insignia cacheada y dónde había
   * caído su recuadro. Un documento que ya estuvo abierto vuelve con su página
   * y su posición; uno nuevo vuelve sin ninguna, que es lo que pide el ID-22.
   */
  record(document: DocumentInHand): Promise<RecentDocument>;
  /** Quita una fila. Ver [`forget`]. */
  forget(id: string): Promise<void>;
  /** Vacía la lista. Es el «Vaciar la lista» de Preferencias. */
  clear(): Promise<void>;
}

/**
 * Los recientes que viven solo durante la sesión y se olvidan al cerrar. Es el
 * doble de las pruebas de la bandeja, que así corren sin backend.
 *
 * Imita la única regla del backend que la ventana nota: una fila que vuelve a
 * anotarse **conserva su recuadro** si la nueva no trae ninguno.
 */
export function inMemoryRecents(initial: readonly RecentDocument[] = []): RecentsStore {
  let entries = initial.slice(0, CAPACITY);
  return {
    list: async () => entries,
    record: async (document) => {
      const previous = entries.find((entry) => entry.id === document.id);
      const noted: RecentDocument = {
        id: document.id,
        name: document.name,
        badge: document.badge,
        modified: document.modified,
        // De una fila que ya estaba se conserva cuándo se usó; una nueva nace
        // ahora. Y lo que el portal acaba de conceder responde, por
        // definición.
        lastUsed: previous?.lastUsed ?? Math.floor(Date.now() / 1000),
        available: previous?.available ?? true,
        placement: document.placement ?? previous?.placement ?? null,
      };
      entries = record(entries, noted);
      return noted;
    },
    forget: async (id) => {
      entries = forget(entries, id);
    },
    clear: async () => {
      entries = [];
    },
  };
}
