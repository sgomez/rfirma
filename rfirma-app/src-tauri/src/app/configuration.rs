//! **Lo que el usuario elige y la aplicación obedece**, del disco a la ventana
//! y de vuelta.
//!
//! Las dos direcciones son asimétricas a propósito: sale todo, entra casi todo.
//! La carpeta de destino **no entra por aquí**: lo que la ventana manda en ese
//! campo se ignora, y quien la mueve es [`choose_destination`], que recibe la
//! carpeta que el selector de directorio concedió y no una ruta escrita por la
//! ventana (ID-65, ADR-0011).

use std::sync::Mutex;

use crate::commands::views::ConfigurationView;
use crate::commands::Failure;
use crate::destination::DestinationFolder;
use crate::memory::{Configuration, Memory};
use crate::signing::Language;

/// El idioma que nombra una etiqueta corta.
///
/// Lo que no reconozcamos cae en castellano, que es el idioma del documento
/// administrativo corriente, y no en un `panic`.
pub fn language_of(tag: &str) -> Language {
    match tag {
        "ca" => Language::Catalan,
        "eu" => Language::Basque,
        "gl" => Language::Galician,
        "va" => Language::Valencian,
        "en" => Language::English,
        _ => Language::Spanish,
    }
}

/// **Caso de uso.** Cómo se ve desde la ventana la configuración que hay
/// guardada.
pub fn shown(
    configuration: &Configuration,
    documents_folder: &std::path::Path,
) -> ConfigurationView {
    let folder = super::chosen_folder(configuration, documents_folder.to_path_buf());
    ConfigurationView {
        language: configuration.language.tag().to_owned(),
        destination: folder.name().to_owned(),
        remember_visible_signature: configuration.remember_visible_signature,
        remember_activity: configuration.remember_activity,
        theme: configuration.theme,
    }
}

/// **Caso de uso.** Guarda lo que el usuario acaba de elegir.
///
/// Actualiza la copia viva **y** el fichero, en ese orden, y las dos cosas o
/// ninguna: si la escritura falla, la copia se deja como estaba, porque una
/// ventana que enseña un ajuste que el disco no tiene miente en la sesión
/// siguiente.
///
/// El borrado del estado al apagar «Recordar mi actividad» **no está aquí**:
/// lo hace [`Memory::remember_configuration`], que es donde no se puede olvidar
/// (ADR-0010).
pub fn write(
    memory: &Memory,
    live: &Mutex<Configuration>,
    chosen: &ConfigurationView,
) -> Result<(), Failure> {
    let mut live = super::lock(live);
    let next = merged(&live, chosen);
    memory.remember_configuration(&next)?;
    *live = next;
    Ok(())
}

/// **Caso de uso.** Guarda la carpeta que el selector de directorio acaba de
/// conceder, y la devuelve por su nombre (ID-65).
///
/// Vive aparte de [`write`] porque el destino **no viaja en
/// [`ConfigurationView`]**: lo que cruza de vuelta es un nombre, y una ruta
/// escrita por la ventana no sería una carpeta concedida por el portal sino una
/// que la ventana se ha inventado. Aquí la ruta entra desde el diálogo que abrió
/// el propio backend, y sale su último segmento (ADR-0011).
///
/// La carpeta **no se comprueba aquí**: lo que el selector concede existe por
/// haberlo concedido, y si deja de existir quien lo cuenta es
/// [`crate::app::documents::where_it_lands`], en el pie y antes de firmar
/// (ID-67).
pub fn choose_destination(
    memory: &Memory,
    live: &Mutex<Configuration>,
    folder: DestinationFolder,
) -> Result<String, Failure> {
    let mut live = super::lock(live);
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

/// **Caso de uso.** Olvida lo acumulado: los recientes y el certificado.
///
/// Es «Vaciar la lista» y también lo que arrastra apagar «Recordar mi
/// actividad» (ID-34). No toca ningún interruptor: la configuración se queda
/// donde estaba, y por eso recibe solo la memoria y no la copia viva.
pub fn forget_activity(memory: &Memory) -> Result<(), Failure> {
    memory.forget_activity()?;
    Ok(())
}

/// Lo elegido, encima de lo guardado.
///
/// Vive aparte de la orden porque es la única decisión que hay dentro —qué
/// campos manda la ventana y cuáles no— y así se puede comprobar sin montar un
/// entorno de Tauri.
pub fn merged(live: &Configuration, chosen: &ConfigurationView) -> Configuration {
    Configuration {
        language: language_of(&chosen.language),
        // El destino no viaja de vuelta: la ventana lo enseña, no lo elige.
        destination: live.destination.clone(),
        remember_visible_signature: chosen.remember_visible_signature,
        remember_activity: chosen.remember_activity,
        theme: chosen.theme,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::{choose_destination, forget_activity, language_of, merged, shown, write};
    use crate::app::fixtures::a_memory;
    use crate::commands::views::ConfigurationView;
    use crate::memory::{Configuration, Theme};
    use crate::signing::Language;

    /// Lo elegido llega al disco **y** a la copia viva, en el mismo paso: una
    /// ventana que enseña un ajuste que el disco no tiene miente en la sesión
    /// siguiente.
    #[test]
    fn what_was_chosen_lands_on_the_disk_and_on_the_live_copy() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let live = Mutex::new(Configuration::default());
        let chosen = ConfigurationView {
            language: "en".to_owned(),
            destination: "Documentos".to_owned(),
            remember_visible_signature: false,
            remember_activity: true,
            theme: Theme::Dark,
        };

        write(&memory, &live, &chosen).expect("deberia guardarse");

        assert_eq!(
            live.lock().expect("sin envenenar").language,
            Language::English
        );
        assert_eq!(
            memory
                .configuration()
                .expect("deberia leerse lo guardado")
                .value()
                .theme,
            Theme::Dark
        );
    }

    /// Olvidar la actividad se lleva lo acumulado —el certificado y los
    /// recientes— y **no** toca ningún interruptor: los ajustes siguen donde
    /// estaban (ID-34).
    #[test]
    fn forgetting_the_activity_keeps_the_settings() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let settings = Configuration {
            theme: Theme::Dark,
            ..Configuration::default()
        };
        memory
            .remember_configuration(&settings)
            .expect("deberia guardarse");
        memory
            .remember_state(
                &settings,
                &crate::memory::State {
                    certificate: Some(crate::pkcs11::CertificateRef::new(
                        "/usr/lib/softhsm/libsofthsm2.so",
                        "rfirma-test",
                        "Certificado de pruebas",
                        vec![0x01],
                    )),
                    ..Default::default()
                },
            )
            .expect("deberia guardarse");

        forget_activity(&memory).expect("deberia olvidarse");

        assert!(
            memory
                .state()
                .expect("deberia leerse")
                .value()
                .certificate
                .is_none(),
            "lo acumulado se olvida"
        );
        assert_eq!(
            memory
                .configuration()
                .expect("deberia leerse")
                .value()
                .theme,
            Theme::Dark,
            "los ajustes no los toca"
        );
    }

    /// Sin destino elegido manda la carpeta de documentos, y sale por su
    /// nombre: la ruta se queda de este lado (ADR-0011).
    #[test]
    fn the_configuration_shows_the_destination_folder_by_its_name() {
        let view = shown(
            &Configuration::default(),
            std::path::Path::new("/home/quien/Documentos"),
        );

        assert_eq!(view.destination, "Documentos");
        assert!(!view.destination.contains('/'));
    }

    /// La ventana no elige la carpeta —bajo el arenero hay una sola—, así que
    /// lo que mande en ese campo no puede reescribir lo guardado.
    #[test]
    fn writing_the_configuration_never_moves_the_destination_folder() {
        let live = Configuration {
            destination: Some(crate::destination::DestinationFolder::at(
                "/home/quien/Documentos/Firmados",
            )),
            ..Configuration::default()
        };
        let chosen = ConfigurationView {
            language: "en".to_owned(),
            destination: "Otra".to_owned(),
            remember_visible_signature: false,
            remember_activity: true,
            theme: Theme::Dark,
        };

        let next = merged(&live, &chosen);

        assert_eq!(
            next.destination, live.destination,
            "el destino no lo elige la ventana"
        );
        assert_eq!(next.language, Language::English);
        assert!(!next.remember_visible_signature);
        assert_eq!(next.theme, Theme::Dark);
    }

    /// Un tema desconocido no puede tumbar la lectura de los ajustes: lo que
    /// hay guardado es un valor cerrado, y el catálogo de la ventana es el
    /// mismo. Aquí solo se comprueba el viaje de ida y vuelta.
    #[test]
    fn the_theme_survives_the_round_trip_to_the_window() {
        for theme in [Theme::System, Theme::Light, Theme::Dark] {
            let configuration = Configuration {
                theme,
                ..Configuration::default()
            };
            let view = shown(
                &configuration,
                std::path::Path::new("/home/quien/Documentos"),
            );

            assert_eq!(merged(&configuration, &view).theme, theme);
        }
    }

    #[test]
    fn the_language_of_the_window_picks_the_labels_of_the_box() {
        assert_eq!(language_of("ca"), Language::Catalan);
        assert_eq!(language_of("en"), Language::English);
        // Lo que no reconozcamos cae en castellano, que es el idioma del
        // documento administrativo corriente, y no en un panic.
        assert_eq!(language_of("de"), Language::Spanish);
    }

    /// La carpeta elegida llega al disco **y** a la copia viva, y vuelve por su
    /// **nombre**: la ruta se queda de este lado (ID-65, ADR-0011).
    #[test]
    fn the_chosen_folder_is_remembered_and_comes_back_by_its_name() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let live = Mutex::new(Configuration::default());

        let name = choose_destination(
            &memory,
            &live,
            crate::destination::DestinationFolder::at("/run/user/1000/doc/1e8b/Firmados"),
        )
        .expect("deberia guardarse");

        assert_eq!(name, "Firmados");
        assert!(!name.contains('/'), "la ruta no cruza");
        assert_eq!(
            super::super::lock(&live).destination,
            Some(crate::destination::DestinationFolder::at(
                "/run/user/1000/doc/1e8b/Firmados"
            )),
            "la copia viva se entera en el mismo paso"
        );
    }

    /// Elegir carpeta **no toca ningún otro ajuste**: es una fila de
    /// Preferencias, no un guardado de la pantalla entera.
    #[test]
    fn choosing_a_folder_leaves_the_other_settings_alone() {
        let home = tempfile::tempdir().expect("deberia haber directorio temporal");
        let memory = a_memory(home.path());
        let live = Mutex::new(Configuration {
            language: Language::English,
            theme: Theme::Dark,
            remember_activity: false,
            ..Configuration::default()
        });

        choose_destination(
            &memory,
            &live,
            crate::destination::DestinationFolder::at("/tmp/Firmados"),
        )
        .expect("deberia guardarse");

        let after = super::super::lock(&live);
        assert_eq!(after.language, Language::English);
        assert_eq!(after.theme, Theme::Dark);
        assert!(!after.remember_activity);
    }
}
