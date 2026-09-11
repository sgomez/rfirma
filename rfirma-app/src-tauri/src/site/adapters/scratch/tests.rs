use super::*;

use std::fs::OpenOptions;

#[test]
fn the_folder_carries_the_role_prefix() {
    let temp = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");

    let folder = own_folder(temp.path(), "site").expect("deberia poder crearse la carpeta");

    let name = folder
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .expect("la carpeta tiene nombre");
    assert!(name.starts_with("rfirma-site-"));
    assert!(folder.path().is_dir());
}

#[test]
fn sweep_erases_a_folder_with_no_lock_held() {
    let temp = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let folder = temp.path().join("rfirma-site-abandoned");
    std::fs::create_dir_all(&folder).expect("deberia poder crearse la carpeta abandonada");

    sweep(temp.path(), &["site", "desktop"]);

    assert!(!folder.exists());
}

#[test]
fn sweep_does_not_erase_a_folder_whose_lock_another_descriptor_holds() {
    let temp = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let folder = temp.path().join("rfirma-site-live");
    std::fs::create_dir_all(&folder).expect("deberia poder crearse la carpeta viva");
    let lock_path = folder.join(LOCK_FILE_NAME);
    let held = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&lock_path)
        .expect("deberia poder abrirse el fichero de cerrojo");
    let locked = unsafe { libc::flock(held.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    assert_eq!(locked, 0, "el otro descriptor deberia tomar el cerrojo");

    sweep(temp.path(), &["site", "desktop"]);

    assert!(folder.exists());
}

#[test]
fn sweep_does_not_touch_folders_of_other_prefixes() {
    let temp = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let untouched = temp.path().join("rfirma-other-abandoned");
    std::fs::create_dir_all(&untouched).expect("deberia poder crearse la carpeta ajena");

    sweep(temp.path(), &["site", "desktop"]);

    assert!(untouched.exists());
}
