/**
 * Los ajustes que la aplicación recuerda, en el lado de la interfaz.
 *
 * Son un subconjunto de `memory::Configuration`: el idioma no está aquí porque
 * ya lo lleva `LanguagePreference` (#55), y la **ruta** de la carpeta de
 * destino tampoco, porque bajo el sandbox la aplicación escribe en ella pero
 * la única palabra que tiene de ella es su último segmento. Enseñar la ruta
 * donde se puede y el nombre donde no sería la misma pantalla contando cosas
 * distintas según el empaquetado (ADR-0011).
 */
import type { Theme } from "./theme";

export interface Preferences {
  /**
   * El tema de la ventana. Ver [`Theme`]. Está aquí y no en un puerto aparte
   * —al contrario que el idioma— porque nació con este diálogo: no hay ningún
   * otro sitio de la aplicación que lo lea.
   */
  theme: Theme;
  /**
   * El **nombre** de la carpeta donde cae el documento firmado —su último
   * segmento—, nunca su ruta.
   */
  destination: string;
  /**
   * Si Preferencias puede ofrecer «Junto al documento original» (ID-184). La
   * contesta el entorno —si sabe devolver la ruta real del documento—, no el
   * usuario: se lee, **no se guarda** al escribir, igual que `destination`.
   */
  offersOriginalFolder: boolean;
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
  /**
   * «Avisarme cuando haya una versión nueva» (ID-180). **Siempre visible y
   * sin condición**: no se detecta si alguien gestiona la instalación. No
   * apaga la comprobación —esa la sigue haciendo el backend cada 24 h—, solo
   * si la ventana enseña la franja con lo que contestó.
   */
  notifyNewVersion: boolean;
  /**
   * Si el aviso del primer arranque (CA local y permiso de red local, #365)
   * ya se ha descartado. No es un ajuste de Preferencias: no hay fila que lo
   * muestre, solo se lee al arrancar y se escribe una vez, al pulsar
   * «Entendido».
   */
  trustNoticeSeen: boolean;
}

/**
 * De dónde salen los ajustes y a dónde vuelven.
 *
 * Puerto, y no una llamada a Tauri, por lo mismo que `LanguagePreference`: la
 * ventana no conoce a Tauri, y quien elige entre `tauriPreferences` y el doble
 * de memoria es `main.tsx` (ID-75). Debajo son `read_configuration` y
 * `write_configuration`, que pasan por `memory::Memory::remember_configuration`
 * —el único sitio donde el borrado del estado al apagarse «Recordar mi
 * actividad» no se puede olvidar—.
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
  /**
   * Abre el **selector de directorio** del sistema y guarda la carpeta que
   * conceda, devolviéndola por su **nombre**; `null` si se cerró sin elegir.
   *
   * Va aquí y no en un puerto aparte porque es el otro medio de escribir un
   * ajuste, y el único que la ventana no puede resolver sola: el diálogo lo
   * abre Rust (ID-65), así que la ventana no manda ninguna ruta —no la
   * conoce— y lo que recibe de vuelta es lo mismo que enseña.
   */
  chooseFolder(): Promise<string | null>;
}

/**
 * Los ajustes mientras no hay dónde guardarlos: viven durante la sesión y se
 * olvidan al cerrar. Sirven también de doble en las pruebas.
 */
export function inMemoryPreferences(
  initial: Preferences,
  onForget: () => void = () => {},
  folder: () => string | null = () => null,
): PreferencesStore {
  let preferences = initial;
  return {
    read: async () => preferences,
    save: async (next) => {
      preferences = next;
    },
    forgetActivity: async () => onForget(),
    chooseFolder: async () => {
      const chosen = folder();
      if (chosen !== null) {
        preferences = { ...preferences, destination: chosen };
      }
      return chosen;
    },
  };
}
