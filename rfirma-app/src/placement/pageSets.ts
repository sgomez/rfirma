//! El recuadro en espacio de usuario PDF, el conjunto de páginas, los modos de páginas y las operaciones puras sobre ellos, con la colocación guardada repartida en los modos. Sin React.

/** El recuadro en espacio de usuario PDF, con las esquinas ya ordenadas. */
export interface UserSpaceRect {
  /** Esquina inferior izquierda, eje X. */
  x0: number;
  /** Esquina inferior izquierda, eje Y. */
  y0: number;
  /** Esquina superior derecha, eje X. */
  x1: number;
  /** Esquina superior derecha, eje Y. */
  y1: number;
}

/**
 * En qué páginas se estampa el recuadro.
 *
 * Cruza tal cual lo lee `signing::placement::PageSet`: la palabra `"all"`, o el
 * registro `{ only: [1, 3] }` con las páginas **1-based, ordenadas y sin
 * repetir**. No hay una tercera forma, y la gramática de lo que se teclea
 * (`1,2-3,10-20`) se traduce a esta antes de cruzar.
 */
export type PageSet = "all" | { only: number[] };

/**
 * El recuadro colocado: **dónde y en qué páginas**.
 *
 * Sustituye al `SignaturePlacement { page, rect }` de v0.2. Es un registro
 * llano y no una unión de un brazo: un `kind` que nunca discrimina es ruido, y
 * el día que entre otra rama de colocación, `rect` y `pages` tendrán que
 * **desaparecer**, no convivir con ella.
 *
 * **«Colocado» no es una bandera: es tener al menos una página sellada**. Por eso no existe una
 * colocación con el conjunto vacío: quitar la última página devuelve `null`, que es exactamente
 * el estado del PDF recién abierto.
 */
export interface Placement {
  rect: UserSpaceRect;
  pages: PageSet;
}

/**
 * Cuál de las tres opciones del panel manda sobre el conjunto.
 *
 * El visor no la elige —vive en el panel— pero la necesita para dos cosas: la cuarta
 * redacción del botón («Colocar el sello aquí» cuando se sellan todas) y que con `Solo 1
 * página` o `Todas las páginas` una página ya sellada **no ofrezca pastilla**, porque no queda
 * nada que ofrecer.
 */
export type PageChoice = "single" | "these" | "all";

/** ¿Esta página lleva recuadro? */
export function sealsPage(pages: PageSet, page: number): boolean {
  return pages === "all" || pages.only.includes(page);
}

/**
 * Las páginas que el conjunto nombra en un documento de `pageCount`.
 *
 * `"all"` y la lista completa dan lo mismo, igual que en `PageSet::resolve`.
 */
export function sealedPages(pages: PageSet, pageCount: number): number[] {
  if (pages !== "all") return pages.only;
  return Array.from({ length: Math.max(0, pageCount) }, (_, index) => index + 1);
}

/**
 * El conjunto ordenado y sin repetir, o `null` si no queda ninguna página.
 *
 * `null` no es un fallo. Sin páginas no hay colocación, y quien lo
 * reciba tiene que **borrar el recuadro**, no guardar un conjunto vacío —que el
 * puente leería como «la última página»—.
 */
export function pageSetOf(pages: Iterable<number>): PageSet | null {
  const only = [...new Set(pages)].sort((a, b) => a - b);
  return only.length === 0 ? null : { only };
}

/** La primera página del conjunto: la que el visor abre y la que mide el panel. */
export function firstSealedPage(placement: Placement | null): number | null {
  if (placement === null) return null;
  if (placement.pages === "all") return 1;
  return placement.pages.only[0] ?? null;
}

/** Añade una página al conjunto. Con «todas» ya estaba dentro y no cambia nada. */
export function sealing(placement: Placement, page: number): Placement {
  if (placement.pages === "all") return placement;
  const pages = pageSetOf([...placement.pages.only, page]);
  return pages === null ? placement : { ...placement, pages };
}

/**
 * Quita una página del conjunto, y con la última **quita la colocación entera**.
 *
 * `"all"` se resuelve antes de restar, que es para lo que hace falta
 * `pageCount`: quitar una de «todas» deja a las demás nombradas una a una.
 */
export function unsealing(placement: Placement, page: number, pageCount: number): Placement | null {
  const rest = sealedPages(placement.pages, pageCount).filter((sealed) => sealed !== page);
  const pages = pageSetOf(rest);
  return pages === null ? null : { ...placement, pages };
}

/**
 * El conjunto que guarda **cada opción** del bloque «Colocación».
 *
 * Las tres opciones no se turnan sobre un mismo conjunto: cada una recuerda el
 * suyo, y elegir otra **no reescribe la que dejas**. Sin esto, sellar la 2 en
 * `Solo 1 página` se sumaba a la 1 en vez de sustituirla, y volver a `Estas
 * páginas` traía lo que hubiera dejado la opción anterior en vez del rango que
 * se tecleó allí.
 *
 * `all` no necesita hueco: su conjunto es la palabra `"all"` y no hay nada que
 * recordar. `single` guarda **un número** y no un `PageSet` porque una página
 * es lo único que esa opción puede llegar a nombrar; el tipo lo dice mejor que
 * una invariante escrita al lado.
 *
 * El **recuadro es uno solo** y no vive aquí: es el mismo rectángulo mirado
 * desde las tres opciones, y cambiar de opción no lo mueve.
 */
export interface PageSets {
  single: number | null;
  these: PageSet | null;
}

/** El documento recién abierto: ninguna opción ha nombrado todavía una página. */
export const NO_PAGE_SETS: PageSets = { single: null, these: null };

/** El conjunto de la opción activa, que es el único que manda sobre la firma. */
export function pagesOf(sets: PageSets, choice: PageChoice): PageSet | null {
  if (choice === "all") return "all";
  if (choice === "these") return sets.these;
  return sets.single === null ? null : { only: [sets.single] };
}

/**
 * La colocación que ve el resto de la ventana: el recuadro compartido y el
 * conjunto de la opción activa.
 *
 * Colocado sigue siendo tener páginas, solo que ahora «tener
 * páginas» se pregunta **por opción**: con el recuadro puesto y `Estas páginas`
 * sin rango, no hay colocación aunque `single` sí tenga la suya.
 */
export function placementOf(
  rect: UserSpaceRect | null,
  sets: PageSets,
  choice: PageChoice,
): Placement | null {
  if (rect === null) return null;
  const pages = pagesOf(sets, choice);
  return pages === null ? null : { rect, pages };
}

/** Guarda `pages` en la opción activa. Las otras dos **no se tocan**. */
export function storing(
  sets: PageSets,
  choice: PageChoice,
  pages: PageSet | null,
  pageCount: number,
): PageSets {
  // «Todas» no tiene conjunto que guardar: es la palabra, siempre la misma.
  if (choice === "all") return sets;
  if (choice === "these") return { ...sets, these: pages };
  return { ...sets, single: pages === null ? null : (sealedPages(pages, pageCount)[0] ?? null) };
}

/**
 * La opción que se activa, sembrada **solo si nunca tuvo conjunto propio**.
 *
 * Es la mitad que sigue debiéndose a la ficha: estrenar `Estas páginas` viniendo
 * de `Solo 1 página` = 3 arranca con `3` escrito. Lo que ya no ocurre es lo
 * contrario —volver a una opción que ya se usó trae **lo suyo**, no lo de la
 * anterior—, y por eso la siembra mira primero si hay algo guardado.
 *
 * `fallback` es la página que se está mirando: la única respuesta razonable
 * cuando `Solo 1 página` se estrena sin nada colocado en ninguna parte.
 */
export function activating(
  sets: PageSets,
  choice: PageChoice,
  previous: PageSet | null,
  pageCount: number,
  fallback: number,
): PageSets {
  if (choice === "all") return sets;
  if (choice === "these") {
    return sets.these === null ? { ...sets, these: previous } : sets;
  }
  if (sets.single !== null) return sets;
  const first = previous === null ? null : (sealedPages(previous, pageCount)[0] ?? null);
  return { ...sets, single: first ?? fallback };
}

/**
 * La colocación **entera**, tal y como la guarda la ventana.
 *
 * Un rectángulo, tres conjuntos —uno por opción— y cuál de ellas manda. Lo que
 * cruza a firmar es el `Placement` que sale de las tres, no esto: aquí vive el
 * estado de la interfaz, y ahí fuera solo se puede firmar en un sitio.
 */
export interface Placing {
  rect: UserSpaceRect | null;
  sets: PageSets;
  choice: PageChoice;
}

/**
 * La colocación guardada en la fila, repartida en las tres opciones.
 *
 * La opción activa es **la que explica el conjunto sin inventar nada**: una
 * página sola es `Solo 1 página`, la palabra `"all"` es `Todas las páginas` y
 * cualquier otra cosa es `Estas páginas`. Las demás arrancan vacías a propósito
 * —no se rellenan «por si acaso»— para que la primera vez que se elijan se
 * siembren de esta, que es lo que pide la ficha.
 */
export function placingFrom(placement: Placement | null, pageCount: number): Placing {
  if (placement === null) return { rect: null, sets: NO_PAGE_SETS, choice: "single" };
  const { rect, pages } = placement;
  if (pages === "all") return { rect, sets: NO_PAGE_SETS, choice: "all" };
  const only = sealedPages(pages, pageCount);
  if (only.length === 1 && only[0] !== undefined) {
    return { rect, sets: { single: only[0], these: null }, choice: "single" };
  }
  return { rect, sets: { single: null, these: pages }, choice: "these" };
}
