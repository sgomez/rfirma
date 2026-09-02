export { CATALOGS, LANGUAGES } from "./locales";

import { LANGUAGES } from "./locales";

/**
 * Los idiomas de la aplicación son **cinco** —`es`, `ca`, `eu`, `gl`, `en`—, y
 * de ellos se publican los que están al 100 % (ID-123, ID-124).
 *
 * Ni la lista ni los catálogos se escriben aquí: salen de `locales/index.ts`,
 * que genera `tools/po-import.mjs` con **los idiomas cuyo `.po` llegó al
 * 100 %**. Un idioma a medias no tiene fichero, así que no puede aparecer en
 * el desplegable: la regla de publicación del ADR-0009 deja de ser algo que
 * alguien comprueba y pasa a ser irrepresentable.
 *
 * El valencià salió en v0.3: `Intl.PluralRules("va")` se resuelve a `und`, con
 * una sola categoría, así que ese catálogo estaba roto para plurales. Las
 * etiquetas son las de `Language::tag` del backend (`signing/language.rs`); si
 * cambia una, cambia en los dos sitios.
 */
export type LanguageTag = (typeof LANGUAGES)[number];

/** El idioma al que cae todo lo que falte. */
export const FALLBACK_LANGUAGE: LanguageTag = "es";

/** Si `value` es una de las etiquetas publicadas. */
export function isLanguageTag(value: string): value is LanguageTag {
  return (LANGUAGES as readonly string[]).includes(value);
}
