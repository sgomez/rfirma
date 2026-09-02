import i18next, { type i18n as I18n } from "i18next";
import { initReactI18next } from "react-i18next";
import { CATALOGS, FALLBACK_LANGUAGE, LANGUAGES, type LanguageTag } from "./languages";

/** El único espacio de nombres: el catálogo es uno y cabe entero en memoria. */
export const NAMESPACE = "translation";

const resources = Object.fromEntries(LANGUAGES.map((tag) => [tag, { [NAMESPACE]: CATALOGS[tag] }]));

/**
 * Una instancia de i18next con los catálogos publicados ya dentro.
 *
 * Dos decisiones que no son las de por omisión y conviene no deshacer:
 *
 * - **Sin `i18next-browser-languagedetector`**. El idioma no se olfatea del
 *   navegador: es una preferencia guardada (ID-02), y quien la lee es
 *   `LanguageProvider`. En la primera ejecución sale del locale del sistema
 *   cotejado contra los publicados, y de eso se encarga el backend.
 * - **`returnEmptyString: false`**, que es lo que hace caer al castellano las
 *   claves vacías de los cuatro idiomas sin traducir (ADR-0009). Con el valor
 *   por omisión, i18next daría la cadena vacía por buena y la interfaz saldría
 *   en blanco en lugar de en español.
 *
 * Los recursos van en línea, así que `init` termina de forma síncrona y no
 * hace falta esperar a nada antes de pintar.
 */
export function createI18n(language: LanguageTag = FALLBACK_LANGUAGE): I18n {
  const instance = i18next.createInstance();
  instance.use(initReactI18next).init({
    lng: language,
    fallbackLng: FALLBACK_LANGUAGE,
    supportedLngs: [...LANGUAGES],
    defaultNS: NAMESPACE,
    resources,
    returnEmptyString: false,
    // React ya escapa lo que pinta; escapar aquí duplicaría las entidades.
    interpolation: { escapeValue: false },
  });
  return instance;
}
