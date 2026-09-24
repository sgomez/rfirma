//! Pruebas de la carpeta en la que se abre el diálogo de sede, al guardar y al cargar.

use std::path::{Path, PathBuf};

use crate::site::application::errand::*;

fn saving_from(declared: Option<&str>) -> SavingConsent {
    SavingConsent {
        data: Vec::new(),
        title: None,
        filename: None,
        extensions: Vec::new(),
        description: None,
        starting_folder: declared.map(str::to_owned),
        signer_der: None,
    }
}

fn loading_from(declared: Option<&str>) -> LoadingConsent {
    LoadingConsent {
        title: None,
        filename: None,
        extensions: Vec::new(),
        description: None,
        starting_folder: declared.map(str::to_owned),
        multiple: false,
        to_sign: None,
    }
}

const HOME: &str = "/home/persona";

#[test]
fn a_save_without_a_declared_folder_opens_in_the_home_directory() {
    let folder = saving_from(None).dialog_folder(Some(Path::new(HOME)));

    assert_eq!(folder, Some(PathBuf::from(HOME)));
}

#[test]
fn a_save_opens_in_the_folder_the_site_declared() {
    let folder = saving_from(Some("/tmp/sede")).dialog_folder(Some(Path::new(HOME)));

    assert_eq!(folder, Some(PathBuf::from("/tmp/sede")));
}

#[test]
fn a_load_without_a_declared_folder_opens_in_the_home_directory() {
    let folder = loading_from(None).dialog_folder(Some(Path::new(HOME)));

    assert_eq!(folder, Some(PathBuf::from(HOME)));
}

#[test]
fn a_load_opens_in_the_folder_the_site_declared() {
    let folder = loading_from(Some("/tmp/sede")).dialog_folder(Some(Path::new(HOME)));

    assert_eq!(folder, Some(PathBuf::from("/tmp/sede")));
}

#[test]
fn without_a_declared_folder_or_a_home_the_dialog_gets_no_folder() {
    assert_eq!(saving_from(None).dialog_folder(None), None);
    assert_eq!(loading_from(None).dialog_folder(None), None);
}
