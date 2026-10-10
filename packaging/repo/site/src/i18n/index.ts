import { ca } from "./ca";
import { en } from "./en";
import type { Dictionary, Key } from "./es";
import { es } from "./es";
import { eu } from "./eu";
import { gl } from "./gl";

export type { Dictionary, Key };

export const locales = ["es", "ca", "eu", "gl", "en"] as const;
export type Locale = (typeof locales)[number];

export const defaultLocale: Locale = "es";

/** El codigo de idioma y region que espera Open Graph, que no es el de la ruta. */
export const openGraphLocales: Record<Locale, string> = {
  es: "es_ES",
  ca: "ca_ES",
  eu: "eu_ES",
  gl: "gl_ES",
  en: "en_GB",
};

export const dictionaries: Record<Locale, Dictionary> = { es, ca, eu, gl, en };

/** Claves cuyo texto es el mismo en los cinco idiomas: nombres propios, formatos, cifras y guiones. */
export const invariantKeys: readonly Key[] = [
  "comparison.head.autofirma",
  "comparison.head.rfirma",
  "comparison.java.label",
  "comparison.kicker",
  "comparison.lang.label",
  "comparison.os.autofirma",
  "comparison.os.label",
  "comparison.updates.rfirma",
  "footer.clienteafirma",
  "footer.col.origin",
  "footer.gpg",
  "footer.releases",
  "footer.repo",
  "hero.card.cert.issuer",
  "hero.card.cert.label",
  "hero.card.cert.value",
  "hero.card.phase.label",
  "hero.card.phase.value",
  "hero.cta.primary",
  "hero.trust.langs",
  "hero.window.title",
  "how.step1.title",
  "install.copied",
  "install.copy",
  "install.kicker",
  "install.macos.title",
  "install.windows.title",
  "lang.aria",
  "lang.ca",
  "lang.en",
  "lang.es",
  "lang.eu",
  "lang.gl",
  "nav.aria",
  "nav.badge.alpha",
  "nav.comparison",
  "nav.github",
  "nav.home.aria",
  "nav.install",
  "nav.manual",
  "nav.transparency",
  "notice.badge",
  "pillars.crypto.title",
  "transparency.kicker",
];

export function isLocale(value: string): value is Locale {
  return (locales as readonly string[]).includes(value);
}

export function translator(locale: Locale): (key: Key) => string {
  const dictionary = dictionaries[locale];
  return (key) => dictionary[key];
}

/** Ruta del manual, que solo existe en castellano: lo enlazan las cinco landings. */
export const manualPath = "/manual/";

/** Ruta de la página de instalación del manual, que enlaza la sección de instalación. */
export const installationManualPath = "/manual/instalacion/";

/** Ruta absoluta de la landing en un idioma: `/` para el castellano y `/<locale>/` para el resto. */
export function localePath(locale: Locale): string {
  return locale === defaultLocale ? "/" : `/${locale}/`;
}
