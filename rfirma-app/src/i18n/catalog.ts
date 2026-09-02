import type es from "./locales/es";

/**
 * La forma del catálogo, tomada del castellano.
 *
 * Sin `as const`: las hojas son `string`, no literales, así que declarar un
 * catálogo como `Catalog` obliga a tener **exactamente** las mismas claves —
 * ni una de menos ni una de más— y eso lo comprueba `tsc`, no una prueba.
 *
 * `locales/es.ts` **no está en el repositorio**: lo genera
 * `tools/po-import.mjs` desde `po/es.po` antes de cada `tsc` (ID-121). Si tu
 * editor dice que no existe, ejecuta `just po`.
 */
export type Catalog = typeof es;

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
