//! Los puertos de Tauri de la configuración: ajustes, idioma y tema de la ventana.

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { FALLBACK_LANGUAGE, isLanguageTag } from "./i18n/languages";
import type { LanguagePreference } from "./i18n/preference";
import type { DestinationMode } from "./preferences/destinationMode";
import type { PreferencesStore } from "./preferences/preferences";
import { DEFAULT_THEME, isTheme, type Theme, type WindowTheme } from "./preferences/theme";

/**
 * La configuración tal como cruza: es `commands::ConfigurationView`, con el
 * destino **por su nombre** y sin una sola ruta (ADR-0011).
 */
interface ConfigurationView {
  language: string;
  destination: string;
  destinationMode: DestinationMode;
  rememberVisibleSignature: boolean;
  rememberActivity: boolean;
  notifyNewVersion: boolean;
  theme: Theme;
  /**
   * **La única pregunta al entorno**: si Preferencias puede ofrecer
   * «Junto al original». La contesta el backend; escribirla no
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
  honourAutomaticSelection: boolean;
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
 * `offersOriginalFolder` tampoco cruza al escribir: la contesta el backend, así que `save` proyecta
 * explícitamente las claves que sí son del
 * contrato de `ConfigurationView`, en vez de mandar `preferences` entero.
 */
export function tauriPreferences(): PreferencesStore {
  return {
    read: async () => {
      const configuration = await readConfiguration();
      return {
        theme: isTheme(configuration.theme) ? configuration.theme : DEFAULT_THEME,
        destination: configuration.destination,
        destinationMode: configuration.destinationMode,
        offersOriginalFolder: configuration.offersTheOriginalFolder,
        rememberVisibleSignature: configuration.rememberVisibleSignature,
        rememberActivity: configuration.rememberActivity,
        notifyNewVersion: configuration.notifyNewVersion,
        setupWizardSeen: configuration.setupWizardSeen,
        consentCountdown: configuration.consentCountdown,
        honourAutomaticSelection: configuration.honourAutomaticSelection,
      };
    },
    save: async (preferences) => {
      const stored = await readConfiguration();
      await writeConfiguration({
        ...stored,
        theme: preferences.theme,
        destinationMode: preferences.destinationMode,
        rememberVisibleSignature: preferences.rememberVisibleSignature,
        rememberActivity: preferences.rememberActivity,
        notifyNewVersion: preferences.notifyNewVersion,
        setupWizardSeen: preferences.setupWizardSeen,
        consentCountdown: preferences.consentCountdown,
        honourAutomaticSelection: preferences.honourAutomaticSelection,
      });
    },
    forgetActivity: () => invoke<void>("forget_activity"),
    chooseFolder: () => invoke<string | null>("choose_destination"),
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

/** El tema de la ventana nativa, que es lo que pinta la barra de título GTK. */
export function tauriWindowTheme(): WindowTheme {
  return (theme) => {
    getCurrentWindow()
      .setTheme(theme)
      .catch((failure) => console.error("no se pudo fijar el tema de la ventana", failure));
  };
}
