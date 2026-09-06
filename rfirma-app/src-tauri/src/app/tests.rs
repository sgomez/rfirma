use std::path::Path;
use std::sync::Mutex;

use super::{chosen_folder, Environment};
use crate::app::fixtures::a_memory;
use crate::destination::DestinationFolder;
use crate::memory::{Configuration, ListedCertificates};

#[test]
fn the_environment_hands_out_a_copy_of_the_live_configuration() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let environment = Environment {
        stores: Vec::new(),
        listed: ListedCertificates::new(),
        documents_folder: home.path().to_path_buf(),
        configuration: Mutex::new(Configuration {
            remember_activity: false,
            ..Configuration::default()
        }),
        memory: a_memory(home.path()),
        rubric: crate::rubric::RubricStore::at(home.path().join("rubric.jpg")),
        installed_certificates: home.path().join("certificates"),
    };

    let copy = environment.configuration();

    assert!(!copy.remember_activity);
    assert!(
        environment.configuration().remember_activity == copy.remember_activity,
        "la copia dice lo mismo que la viva"
    );
}

#[test]
fn the_remembered_folder_is_reused_and_nobody_is_asked_again() {
    let configuration = Configuration {
        destination: Some(DestinationFolder::at("/home/quien/Documentos/Firmados")),
        ..Configuration::default()
    };

    let folder = chosen_folder(&configuration, "/home/quien/Documentos");

    assert_eq!(
        folder.path(),
        Path::new("/home/quien/Documentos/Firmados"),
        "elegida una vez, se reutiliza"
    );
    assert_eq!(folder.name(), "Firmados");
}

#[test]
fn without_a_remembered_folder_the_destination_is_the_documents_folder() {
    let folder = chosen_folder(&Configuration::default(), "/home/quien/Documentos");

    assert_eq!(folder.path(), Path::new("/home/quien/Documentos"));
}
