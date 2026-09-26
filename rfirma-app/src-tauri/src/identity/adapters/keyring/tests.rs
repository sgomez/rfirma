use std::sync::Arc;

use tempfile::tempdir;

use super::*;

/// El llavero de fichero del propio crate `oo7`, en un directorio temporal, sin D-Bus.
fn a_file_backed_keyring() -> (tempfile::TempDir, RealKeyring) {
    let directory = tempdir().expect("crea un directorio temporal");
    let path = directory.path().join("almacen.keyring");
    let secret = oo7::Secret::text("clave de prueba del llavero de fichero");

    let backend = tauri::async_runtime::block_on(oo7::file::Keyring::load(&path, secret))
        .expect("abre el llavero de fichero");

    (
        directory,
        RealKeyring::from_backend(oo7::Keyring::File(Arc::new(backend))),
    )
}

#[test]
fn a_keyring_without_an_item_yet_has_no_pin() {
    let (_directory, keyring) = a_file_backed_keyring();

    assert_eq!(keyring.pin(), Err(KeyringError::PinMissing));
}

#[test]
fn create_pin_can_be_read_back() {
    let (_directory, keyring) = a_file_backed_keyring();

    let created = keyring.create_pin().expect("crea el PIN");
    let read = keyring.pin().expect("lee el PIN creado");

    assert_eq!(created, read);
}

#[test]
fn get_or_create_pin_creates_only_once() {
    let (_directory, keyring) = a_file_backed_keyring();

    let first = keyring.get_or_create_pin().expect("crea el PIN");
    let second = keyring
        .get_or_create_pin()
        .expect("reutiliza el PIN creado");

    assert_eq!(first, second);
}
