/** Los puertos de Tauri de la configuración: ajustes, idioma, destino y la versión publicada. */

import { invoke } from "@tauri-apps/api/core";
import type { ExternalDestinationOpener } from "./desktop/externalDestination";
import { FALLBACK_LANGUAGE, isLanguageTag } from "./i18n/languages";
import type { LanguagePreference } from "./i18n/preference";
import type { PreferencesStore } from "./preferences/preferences";
import { DEFAULT_THEME, isTheme, type Theme } from "./preferences/theme";
import type { Destination, DestinationSource, SignedDocumentOpener } from "./signing/destination";
import type { NewVersion, VersionCheck } from "./updates/newVersion";

/**
 * La configuración tal como cruza: es `commands::ConfigurationView`, con el
 * destino **por su nombre** y sin una sola ruta (ADR-0011).
 */
interface ConfigurationView {
  language: string;
  destination: string;
  rememberVisibleSignature: boolean;
  rememberActivity: boolean;
  notifyNewVersion: boolean;
  theme: Theme;
  /**
   * **La única pregunta al entorno** (ID-184): si Preferencias puede ofrecer
   * «Junto al documento original». La contesta el backend; escribirla no
   * sirve de nada, así que no cruza al revés.
   */
  offersTheOriginalFolder: boolean;
  /**
   * Si el asistente del primer arranque ya se ha visto. Viaja en los dos
   * sentidos: se lee para decidir si el asistente se monta y se escribe una
   * vez, al pulsar «Terminar».
   */
  setupWizardSeen: boolean;
  consentCountdown: boolean;
}

function readConfiguration(): Promise<ConfigurationView> {
  return invoke<ConfigurationView>("read_configuration");
}

function writeConfiguration(configuration: ConfigurationView): Promise<void> {
  return invoke<void>("write_configuration", { configuration });
}

/**
 * Los ajustes, guardados en el disco por `memory::Memory`.
 *
 * Cada escritura **relee** antes de escribir en vez de recordar lo último que
 * mandó: el idioma va por su propio puerto y se guarda en la misma
 * configuración, así que una copia local aquí se quedaría atrás en cuanto
 * alguien cambiara el idioma y devolvería el anterior en la escritura
 * siguiente.
 *
 * El destino que se manda es el que se leyó: la ventana lo enseña y no lo
 * elige —bajo el sandbox hay una sola carpeta—, y el backend lo ignora.
 *
 * `offersOriginalFolder` tampoco cruza al escribir: la contesta el backend
 * (ID-184), así que `save` proyecta explícitamente las claves que sí son del
 * contrato de `ConfigurationView`, en vez de mandar `preferences` entero.
 */
export function tauriPreferences(): PreferencesStore {
  return {
    read: async () => {
      const configuration = await readConfiguration();
      return {
        theme: isTheme(configuration.theme) ? configuration.theme : DEFAULT_THEME,
        destination: configuration.destination,
        offersOriginalFolder: configuration.offersTheOriginalFolder,
        rememberVisibleSignature: configuration.rememberVisibleSignature,
        rememberActivity: configuration.rememberActivity,
        notifyNewVersion: configuration.notifyNewVersion,
        setupWizardSeen: configuration.setupWizardSeen,
        consentCountdown: configuration.consentCountdown,
      };
    },
    save: async (preferences) => {
      const stored = await readConfiguration();
      await writeConfiguration({
        ...stored,
        theme: preferences.theme,
        rememberVisibleSignature: preferences.rememberVisibleSignature,
        rememberActivity: preferences.rememberActivity,
        notifyNewVersion: preferences.notifyNewVersion,
        setupWizardSeen: preferences.setupWizardSeen,
        consentCountdown: preferences.consentCountdown,
      });
    },
    forgetActivity: () => invoke<void>("forget_activity"),
    chooseFolder: () => invoke<string | null>("choose_destination"),
  };
}

/**
 * Dónde caerá el documento que hay delante: `preview_destination`.
 *
 * Lo compone el backend con la misma carpeta comprobada y el mismo
 * `landing_for` con los que va a escribir después, así que el pie enseña lo que
 * va a ocurrir y no una promesa parecida (ID-63, ID-67).
 */
export function tauriDestinations(): DestinationSource {
  return {
    previewFor: (documentId) => invoke<Destination>("preview_destination", { id: documentId }),
  };
}

export function tauriExternalDestinationOpener(): ExternalDestinationOpener {
  return {
    open: (destination) => invoke<void>("open_external_destination", { target: destination }),
  };
}

/**
 * Abrir el PDF firmado y su carpeta: `open_signed_document` y
 * `open_signed_folder`.
 *
 * **No se les manda ninguna ruta**, porque la ventana no tiene ninguna
 * (ADR-0011): lo que abren es el fichero que dejó la última postfirma, que es
 * justo el que el resumen tiene delante. El complemento `opener` se llama desde
 * Rust por lo mismo que el del diálogo (ID-63, ID-85), y debajo es el portal
 * `OpenURI`.
 */
export function tauriSignedDocumentOpener(): SignedDocumentOpener {
  return {
    openDocument: () => invoke<void>("open_signed_document"),
    openFolder: () => invoke<void>("open_signed_folder"),
  };
}

/**
 * El idioma, en la misma configuración que los demás ajustes.
 *
 * Es un puerto aparte porque el idioma se lee **antes** de que haya ventana
 * —`createI18n` lo necesita para el primer pintado— y los ajustes solo se leen
 * al montar la aplicación. Debajo es el mismo fichero.
 */
export function tauriLanguagePreference(): LanguagePreference {
  return {
    read: async () => {
      const { language } = await readConfiguration();
      return isLanguageTag(language) ? language : FALLBACK_LANGUAGE;
    },
    save: async (language) => {
      const stored = await readConfiguration();
      await writeConfiguration({ ...stored, language });
    },
  };
}

/**
 * Si hay una versión nueva publicada.
 *
 * Aquí no hay ni URL ni caché ni comparación de versiones: todo eso es de
 * `app::version`, que es quien pregunta —como mucho una vez cada 24 h— y quien
 * decide que sin red no se dice nada. La orden contesta `null` en los tres
 * casos en que no hay nada que contar, y `null` es lo que llega a la ventana.
 */
export function tauriVersionCheck(): VersionCheck {
  return {
    latest: async () => await invoke<NewVersion | null>("check_for_new_version"),
  };
}
