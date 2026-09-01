import type { Badge, ShownBadge } from "./document";

/**
 * Cuántos se recuerdan. Es el mismo `memory::recents::CAPACITY` del backend:
 * la bandeja no tiene buscador, y si algún día hace falta uno este límite
 * estaba mal.
 */
export const CAPACITY = 10;

/**
 * Un documento de la bandeja, con lo que hace falta para pintar la fila **sin
 * abrirlo**.
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
   * No es una ruta y de él no se puede reconstruir ninguna: bajo el arenero la
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
 * Mete un documento al frente de la lista, desalojando por el final.
 *
 * La identidad es el identificador opaco, así que reabrir uno viejo **lo rescata**
 * en vez de duplicarlo. El orden de la lista es el que manda para el desalojo;
 * `lastUsed` es dato para pintar la fila.
 */
export function record(
  recents: readonly RecentDocument[],
  document: RecentDocument,
): RecentDocument[] {
  const others = recents.filter((entry) => entry.id !== document.id);
  return [document, ...others].slice(0, CAPACITY);
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
 * `LanguagePreference`: quien los guarda es el backend (`memory::State`), y
 * todavía no hay ninguna orden expuesta que los lea ni los escriba. Cuando la
 * haya, su implementación se enchufa en `main.tsx` sin tocar ni la bandeja ni
 * sus pruebas.
 */
export interface RecentsStore {
  /** La lista entera, la más reciente primero, con `available` ya resuelto. */
  list(): Promise<RecentDocument[]>;
  /** Registra un documento recién abierto o recién firmado. */
  record(document: RecentDocument): Promise<void>;
  /** Quita una fila. Ver [`forget`]. */
  forget(id: string): Promise<void>;
  /** Vacía la lista. Es el «Vaciar la lista» de Preferencias. */
  clear(): Promise<void>;
}

/**
 * Los recientes mientras no hay dónde guardarlos: viven durante la sesión y se
 * olvidan al cerrar. Sirven también de doble en las pruebas.
 */
export function inMemoryRecents(initial: readonly RecentDocument[] = []): RecentsStore {
  let entries = initial.slice(0, CAPACITY);
  return {
    list: async () => entries,
    record: async (document) => {
      entries = record(entries, document);
    },
    forget: async (id) => {
      entries = forget(entries, id);
    },
    clear: async () => {
      entries = [];
    },
  };
}
