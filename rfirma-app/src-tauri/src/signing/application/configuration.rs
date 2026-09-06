//! Paso de configuración entre la interfaz y el almacenamiento en disco (ADR-0010, ADR-0011).

use std::sync::Mutex;

use crate::commands::Failure;
use crate::documents::domain::destination::DestinationFolder;
use crate::signing::adapters::views::ConfigurationView;
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::domain::Language;
use crate::Memory;

/// Resuelve el idioma soportado a partir de su código o devuelve castellano por omisión.
pub fn language_of(tag: &str) -> Language {
    match tag {
        "ca" => Language::Catalan,
        "eu" => Language::Basque,
        "gl" => Language::Galician,
        "en" => Language::English,
        _ => Language::Spanish,
    }
}

/// Proyecta la configuración guardada como vista para la ventana.
pub fn shown(
    configuration: &Configuration,
    documents_folder: &std::path::Path,
) -> ConfigurationView {
    let folder = crate::chosen_folder(configuration, documents_folder.to_path_buf());
    ConfigurationView {
        language: configuration.language.tag().to_owned(),
        destination: folder.name().to_owned(),
        remember_visible_signature: configuration.remember_visible_signature,
        remember_activity: configuration.remember_activity,
        notify_new_version: configuration.notify_new_version,
        theme: configuration.theme,
        offers_the_original_folder:
            crate::documents::adapters::portal::the_original_folder_can_be_offered(),
        trust_notice_seen: configuration.trust_notice_seen,
        ask_about_url_handler: configuration.ask_about_url_handler,
    }
}

/// Guarda la configuración elegida en disco y actualiza la copia en memoria viva (ADR-0010).
pub fn write(
    memory: &Memory,
    live: &Mutex<Configuration>,
    chosen: &ConfigurationView,
) -> Result<(), Failure> {
    let mut live = crate::lock(live);
    let next = merged(&live, chosen);
    memory.remember_configuration(&next)?;
    *live = next;
    Ok(())
}

/// Guarda la carpeta destino concedida y devuelve su nombre visible (ADR-0011).
pub fn choose_destination(
    memory: &Memory,
    live: &Mutex<Configuration>,
    folder: DestinationFolder,
) -> Result<String, Failure> {
    let mut live = crate::lock(live);
    let next = Configuration {
        destination: Some(folder),
        ..live.clone()
    };
    memory.remember_configuration(&next)?;
    let name = next
        .destination
        .as_ref()
        .map(|folder| folder.name().to_owned())
        .unwrap_or_default();
    *live = next;
    Ok(name)
}

/// Borra la actividad acumulada conservando las preferencias (ADR-0010).
pub fn forget_activity(memory: &Memory) -> Result<(), Failure> {
    memory.forget_activity()?;
    Ok(())
}

/// Combina la configuración viva con los campos modificables desde la ventana.
pub fn merged(live: &Configuration, chosen: &ConfigurationView) -> Configuration {
    Configuration {
        language: language_of(&chosen.language),
        destination: live.destination.clone(),
        remember_visible_signature: chosen.remember_visible_signature,
        remember_activity: chosen.remember_activity,
        notify_new_version: chosen.notify_new_version,
        theme: chosen.theme,
        trust_notice_seen: chosen.trust_notice_seen,
        ask_about_url_handler: chosen.ask_about_url_handler,
    }
}

#[cfg(test)]
mod tests;
