/**
 * Los ajustes que la aplicación recuerda, en el lado de la interfaz.
 *
 * Son un subconjunto de `memory::Configuration`: el idioma no está aquí porque
 * ya lo lleva `LanguagePreference` (#55), y la **ruta** de la carpeta de
 * destino tampoco, porque bajo el arenero la aplicación escribe en ella pero
 * la única palabra que tiene de ella es su último segmento. Enseñar la ruta
 * donde se puede y el nombre donde no sería la misma pantalla contando cosas
 * distintas según el empaquetado (ADR-0011).
 */
export interface Preferences {
  /**
   * El **nombre** de la carpeta donde cae el documento firmado —su último
   * segmento—, nunca su ruta.
   */
  destination: string;
  /**
   * «Recordar la última configuración de firma visible». Apagado significa
   * **no guardarla**, no guardarla y no aplicarla.
   */
  rememberVisibleSignature: boolean;
  /**
   * «Recordar mi actividad». Cubre los recientes **y** el certificado: son la
   * misma promesa a quien firma en un ordenador compartido. Al apagarse
   * **borra** lo ya recordado, previa confirmación.
   */
  rememberActivity: boolean;
}

/**
 * De dónde salen los ajustes y a dónde vuelven.
 *
 * Puerto, y no una llamada a Tauri, por lo mismo que `LanguagePreference`:
 * quien los guarda es `memory::Memory::remember_configuration`, que además es
 * quien borra el estado al apagarse «Recordar mi actividad», y todavía no hay
 * orden expuesta que lo llame.
 */
export interface PreferencesStore {
  read(): Promise<Preferences>;
  save(preferences: Preferences): Promise<void>;
  /**
   * Olvida lo acumulado: los recientes y el certificado. Es
   * `Memory::forget_activity`, y lo disparan tanto «Vaciar la lista» como
   * apagar «Recordar mi actividad».
   */
  forgetActivity(): Promise<void>;
}

/**
 * Los ajustes mientras no hay dónde guardarlos: viven durante la sesión y se
 * olvidan al cerrar. Sirven también de doble en las pruebas.
 */
export function inMemoryPreferences(
  initial: Preferences,
  onForget: () => void = () => {},
): PreferencesStore {
  let preferences = initial;
  return {
    read: async () => preferences,
    save: async (next) => {
      preferences = next;
    },
    forgetActivity: async () => onForget(),
  };
}
