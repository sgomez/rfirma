use std::sync::Mutex;

use super::{choose_destination, forget_activity, language_of, merged, shown, write, Preferences};
use crate::fixtures::a_memory;
use crate::signing::application::configuration_memory::{Configuration, Theme};
use crate::signing::domain::Language;

#[test]
fn the_configuration_carries_whether_the_original_folder_can_be_offered() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");

    let view = shown(&Configuration::default(), home.path());

    assert_eq!(
        view.offers_the_original_folder,
        !std::path::Path::new("/.flatpak-info").exists()
    );
}

#[test]
fn what_was_chosen_lands_on_the_disk_and_on_the_live_copy() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let live = Mutex::new(Configuration::default());
    let chosen = Preferences {
        language: "en".to_owned(),
        destination: "Documentos".to_owned(),
        remember_visible_signature: false,
        remember_activity: true,
        notify_new_version: true,
        theme: Theme::Dark,
        offers_the_original_folder: false,
        trust_notice_seen: false,
        ask_about_url_handler: true,
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
            &crate::signing::application::state::State {
                certificate: Some(crate::identity::adapters::pkcs11::CertificateRef::new(
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

#[test]
fn the_configuration_shows_the_destination_folder_by_its_name() {
    let view = shown(
        &Configuration::default(),
        std::path::Path::new("/home/quien/Documentos"),
    );

    assert_eq!(view.destination, "Documentos");
    assert!(!view.destination.contains('/'));
}

#[test]
fn writing_the_configuration_never_moves_the_destination_folder() {
    let live = Configuration {
        destination: Some(
            crate::documents::domain::destination::DestinationFolder::at(
                "/home/quien/Documentos/Firmados",
            ),
        ),
        ..Configuration::default()
    };
    let chosen = Preferences {
        language: "en".to_owned(),
        destination: "Otra".to_owned(),
        remember_visible_signature: false,
        remember_activity: true,
        notify_new_version: true,
        theme: Theme::Dark,
        offers_the_original_folder: false,
        trust_notice_seen: false,
        ask_about_url_handler: true,
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

#[test]
fn not_asking_about_the_url_handler_again_travels_back_from_the_window() {
    let live = Configuration::default();
    let chosen = Preferences {
        ask_about_url_handler: false,
        ..shown(&live, std::path::Path::new("/home/quien/Documentos"))
    };

    assert!(live.ask_about_url_handler);
    assert!(!merged(&live, &chosen).ask_about_url_handler);
}

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
    assert_eq!(language_of("de"), Language::Spanish);
    assert_eq!(language_of("va"), Language::Spanish);
}

#[test]
fn the_chosen_folder_is_remembered_and_comes_back_by_its_name() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let live = Mutex::new(Configuration::default());

    let name = choose_destination(
        &memory,
        &live,
        crate::documents::domain::destination::DestinationFolder::at(
            "/run/user/1000/doc/1e8b/Firmados",
        ),
    )
    .expect("deberia guardarse");

    assert_eq!(name, "Firmados");
    assert!(!name.contains('/'), "la ruta no cruza");
    assert_eq!(
        crate::lock(&live).destination,
        Some(
            crate::documents::domain::destination::DestinationFolder::at(
                "/run/user/1000/doc/1e8b/Firmados"
            )
        ),
        "la copia viva se entera en el mismo paso"
    );
}

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
        crate::documents::domain::destination::DestinationFolder::at("/tmp/Firmados"),
    )
    .expect("deberia guardarse");

    let after = crate::lock(&live);
    assert_eq!(after.language, Language::English);
    assert_eq!(after.theme, Theme::Dark);
    assert!(!after.remember_activity);
}
