import type { es } from "./locales/es";

/**
 * La forma del catálogo, tomada del castellano.
 *
 * Sin `as const`: las hojas son `string`, no literales, así que declarar un
 * catálogo como `Catalog` obliga a tener **exactamente** las mismas claves —
 * ni una de menos ni una de más— y eso lo comprueba `tsc`, no una prueba.
 */
export type Catalog = typeof es;

type Untranslated<T> = { [K in keyof T]: T[K] extends string ? "" : Untranslated<T[K]> };

/**
 * Un catálogo **sin traducir**: las mismas claves, y todas vacías. El tipo
 * exige lo segundo, así que una traducción a medias no puede colarse aquí.
 *
 * Los cuatro idiomas cooficiales de v0.1 son de este tipo. Existen así a
 * propósito (ADR-0009): un agente puede generar los seis ficheros, pero nadie
 * va a revisar el euskara antes de v0.1, y traducción sin revisar en una
 * aplicación de firma es peor que su ausencia. Lo que se acota es **cuándo**
 * se rellenan, no cuántas lenguas hay. Al rellenar uno, cambia su tipo a
 * `Catalog` y el idioma aparece solo en el desplegable de Preferencias.
 */
export type UntranslatedCatalog = Untranslated<Catalog>;

/** Cada hoja del catálogo, con su ruta en punto: `errors.technicalDetail`. */
export function catalogKeys(catalog: unknown, prefix = ""): string[] {
  if (typeof catalog !== "object" || catalog === null) return [prefix];
  return Object.entries(catalog).flatMap(([key, value]) =>
    catalogKeys(value, prefix ? `${prefix}.${key}` : key),
  );
}

/** Los textos de cada hoja, en el mismo orden que [`catalogKeys`]. */
export function catalogValues(catalog: unknown): string[] {
  if (typeof catalog !== "object" || catalog === null) return [String(catalog)];
  return Object.values(catalog).flatMap((value) => catalogValues(value));
}

/**
 * Un catálogo está **completo** cuando ninguna de sus hojas está vacía.
 *
 * Es la regla de la ficha de Preferencias: un idioma no aparece en el
 * desplegable si le falta una sola clave, porque caer al castellano a mitad de
 * pantalla no es una degradación aceptable.
 */
export function isComplete(catalog: Catalog): boolean {
  return catalogValues(catalog).every((value) => value.trim() !== "");
}
