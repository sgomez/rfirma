import type { Catalog } from "./catalog";
import { isComplete } from "./catalog";
import { ca } from "./locales/ca";
import { en } from "./locales/en";
import { es } from "./locales/es";
import { eu } from "./locales/eu";
import { gl } from "./locales/gl";
import { va } from "./locales/va";

/**
 * Los seis idiomas, en el orden en que se enseñan.
 *
 * Es la misma lista que `LanguageManager.AFIRMA_DEFAULT_LOCALES` del cliente
 * oficial, y se toma entera: un subconjunto de las lenguas cooficiales no es
 * una decisión técnica sino una asimetría entre lenguas, y en una herramienta
 * de firma ante la Administración se lee como tal (ADR-0009).
 *
 * Las etiquetas son las de `Language::tag` del backend (`signing/language.rs`),
 * que es como se persiste la preferencia. Si cambia una, cambia en los dos
 * sitios.
 */
export const LANGUAGES = ["es", "ca", "eu", "gl", "va", "en"] as const;

export type LanguageTag = (typeof LANGUAGES)[number];

/** El idioma al que cae todo lo que falte. */
export const FALLBACK_LANGUAGE: LanguageTag = "es";

/** Los seis catálogos, por etiqueta. */
export const CATALOGS: Record<LanguageTag, Catalog> = { es, ca, eu, gl, va, en };

/** Si `value` es una de las seis etiquetas. */
export function isLanguageTag(value: string): value is LanguageTag {
  return (LANGUAGES as readonly string[]).includes(value);
}

/**
 * Los idiomas que pueden aparecer en el desplegable de Preferencias: los que
 * tienen **todas** las cadenas. Caer al castellano a mitad de pantalla no es
 * una degradación aceptable, así que un idioma incompleto no se ofrece aunque
 * su catálogo exista (ADR-0009 y `docs/design/preferencias.md`).
 */
export function completeLanguages(): LanguageTag[] {
  return LANGUAGES.filter((tag) => isComplete(CATALOGS[tag]));
}
