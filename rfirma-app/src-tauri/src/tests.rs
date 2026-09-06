use std::path::Path;
use std::sync::Mutex;

use crate::{chosen_folder, Environment};

mod memory;
use crate::documents::domain::destination::DestinationFolder;
use crate::fixtures::a_memory;
use crate::identity::application::listed::ListedCertificates;
use crate::signing::application::configuration_memory::Configuration;

#[test]
fn the_environment_hands_out_a_copy_of_the_live_configuration() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let environment = Environment {
        token: Box::new(crate::fixtures::NoToken),
        stores: Vec::new(),
        listed: ListedCertificates::new(),
        documents_folder: home.path().to_path_buf(),
        configuration: Mutex::new(Configuration {
            remember_activity: false,
            ..Configuration::default()
        }),
        memory: a_memory(home.path()),
        rubric: crate::documents::adapters::rubric::RubricStore::at(home.path().join("rubric.jpg")),
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
