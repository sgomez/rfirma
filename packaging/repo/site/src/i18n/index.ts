import { es } from "./es";
import { ca } from "./ca";
import { eu } from "./eu";
import { gl } from "./gl";
import { en } from "./en";
import type { Dictionary, Key } from "./es";

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
  "comparison.arch.autofirma",
  "comparison.arch.label",
  "comparison.head.aspect",
  "comparison.head.autofirma",
  "comparison.head.rfirma",
  "comparison.kicker",
  "comparison.lang.label",
  "comparison.os.autofirma",
  "comparison.privacy.autofirma",
  "comparison.sede.autofirma",
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
  "mock.cert.chosen.detail",
  "mock.certlist.available",
  "mock.certlist.cert2.detail",
  "mock.certlist.cert2.name",
  "mock.certlist.cert3.detail",
  "mock.certlist.cert3.name",
  "mock.file.signed",
  "mock.file.unsigned",
  "mock.pageOf",
  "mock.panel.certificate",
  "mock.pin.cancel",
  "mock.pin.label",
  "mock.pin.subject",
  "mock.recent2.name",
  "mock.recent3.name",
  "mock.saveIn.dir",
  "mock.stamp.date",
  "mock.stamp.id",
  "mock.stamp.name",
  "mock.summary.format",
  "mock.summary.sig2.detail",
  "mock.summary.sig2.name",
  "mock.zoom",
  "nav.aria",
  "nav.badge.alpha",
  "nav.comparison",
  "nav.features",
  "nav.github",
  "nav.home.aria",
  "nav.install",
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

/** Ruta absoluta de la landing en un idioma: `/` para el castellano y `/<locale>/` para el resto. */
export function localePath(locale: Locale): string {
  return locale === defaultLocale ? "/" : `/${locale}/`;
}
