import { FALLBACK_LANGUAGE, type LanguageTag } from "./languages";

/**
 * De dónde sale y a dónde vuelve el idioma: una **preferencia**, es decir un
 * ajuste que el usuario elige y la aplicación se limita a obedecer
 * (`CONTEXT.md`). No se olfatea del navegador.
 *
 * Es un puerto y no una llamada a Tauri porque quien guarda la configuración
 * es el backend (`memory::Configuration`, ID-31) y todavía no hay orden
 * expuesta que leerla y escribirla; el diálogo de Preferencias trae la
 * implementación de verdad y la enchufa en `main.tsx` sin tocar nada más.
 */
export interface LanguagePreference {
  read(): Promise<LanguageTag>;
  save(language: LanguageTag): Promise<void>;
}

/**
 * La preferencia mientras no hay dónde guardarla: recuerda el idioma durante
 * la sesión y lo olvida al cerrar. Sirve también de doble en las pruebas.
 */
export function inMemoryLanguagePreference(
  initial: LanguageTag = FALLBACK_LANGUAGE,
): LanguagePreference {
  let language = initial;
  return {
    read: async () => language,
    save: async (next) => {
      language = next;
    },
  };
}
