use std::path::Path;

use super::super::{
    certificate_behind, remember_the_certificate, remove_installed, rows_of, ListedCertificates,
};
use super::removal::{FixedPinKeyring, RemovalOutcome};
use super::{CARD, INSTALLED, SOFTOKEN};
use crate::identity::application::tests::{a_certificate_with_id, TestAuthority};
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::store::{Store, StoreClass};
use crate::signing::application::tests::a_memory;

#[test]
fn twin_copies_of_one_certificate_in_one_token_are_one_row_behind_the_first() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let der = TestAuthority::root("GEMELO").der();
    let certificates = [
        a_certificate_with_id("FNMT-ACTIVO-99999999R", 0x01, &der),
        a_certificate_with_id("FNMT-GEMELO-99999999R", 0x04, &der),
    ];
    let listed = ListedCertificates::new();

    let rows = rows_of(
        certificates.to_vec(),
        &home.path().join("certificates"),
        &listed,
        &ListedCertificates::new(),
        &a_memory(home.path()),
    );

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].stores, vec![StoreClass::Card]);
    let chosen = certificate_behind(&certificates, &rows[0].id, &listed).expect("la copia");
    assert_eq!(chosen.reference().cka_id(), Some([0x01].as_slice()));
}

/// El mismo certificado en cada uno de esos almacenes, con su etiqueta propia en cada uno.
fn copies_of(der: &[u8], stores: &[&Store]) -> Vec<TokenCertificate> {
    stores
        .iter()
        .enumerate()
        .map(|(copy, store)| {
            TokenCertificate::new(
                CertificateRef::new(
                    (*store).clone(),
                    "rfirma-test",
                    format!("COPIA-{copy}"),
                    vec![0x01],
                ),
                der.to_vec(),
            )
        })
        .collect()
}

fn a_firefox_profile() -> Store {
    Store::nss(SOFTOKEN, Path::new("/home/ada/.mozilla/firefox/perfil"))
}

fn a_chrome_database() -> Store {
    Store::nss(SOFTOKEN, Path::new("/home/ada/.pki/nssdb"))
}

/// El Almacén de rFirma en `directory`, con su base ya creada.
fn the_installed_store(directory: &Path) -> Store {
    std::fs::create_dir_all(directory).expect("deberia crearse el almacen");
    std::fs::write(directory.join("cert9.db"), b"").expect("deberia crearse la base");
    Store::installed_nss(INSTALLED, directory)
}

#[test]
fn two_copies_of_one_certificate_are_one_row_with_both_stores() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let der = TestAuthority::root("EIDAS CERTIFICADO PRUEBAS - 99999999R").der();

    let rows = rows_of(
        copies_of(&der, &[&a_firefox_profile(), &Store::module(CARD)]),
        &home.path().join("certificates"),
        &ListedCertificates::new(),
        &ListedCertificates::new(),
        &a_memory(home.path()),
    );

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].stores, vec![StoreClass::Card, StoreClass::Firefox]);
}

#[test]
fn the_same_serial_number_from_two_issuers_is_two_rows() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let one = TestAuthority::root_with_serial("EMISORA UNO", 1234).as_certificate("UNO");
    let other = TestAuthority::root_with_serial("EMISORA OTRA", 1234).as_certificate("OTRO");
    assert_eq!(one.serial_number(), other.serial_number());

    let rows = rows_of(
        vec![one, other],
        &home.path().join("certificates"),
        &ListedCertificates::new(),
        &ListedCertificates::new(),
        &a_memory(home.path()),
    );

    assert_eq!(rows.len(), 2);
}

#[test]
fn with_nothing_remembered_the_row_signs_with_the_copy_of_the_preferred_store() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let der = TestAuthority::root("EIDAS CERTIFICADO PRUEBAS - 99999999R").der();
    let copies = copies_of(
        &der,
        &[
            &a_chrome_database(),
            &a_firefox_profile(),
            &Store::nss(SOFTOKEN, Path::new("/home/ada/.local/share/otro-perfil")),
            &the_installed_store(&home.path().join("certificates")),
            &Store::module(CARD),
        ],
    );
    let listed = ListedCertificates::new();

    let rows = rows_of(
        copies.clone(),
        &home.path().join("certificates"),
        &listed,
        &ListedCertificates::new(),
        &a_memory(home.path()),
    );

    assert_eq!(
        rows[0].stores,
        vec![
            StoreClass::Card,
            StoreClass::Installed,
            StoreClass::Nssdb,
            StoreClass::Firefox,
            StoreClass::Chrome
        ]
    );
    assert_eq!(rows[0].store, StoreClass::Card);
    assert!(!rows[0].remembered);
    let chosen = certificate_behind(&copies, &rows[0].id, &listed).expect("la copia");
    assert_eq!(chosen.reference(), copies[4].reference());
}

#[test]
fn the_windows_copy_is_preferred_to_the_same_card_seen_through_pkcs11() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let der = TestAuthority::root("EIDAS CERTIFICADO PRUEBAS - 99999999R").der();

    let rows = rows_of(
        copies_of(
            &der,
            &[&Store::module(CARD), &Store::module("cng:CurrentUser/MY")],
        ),
        &home.path().join("certificates"),
        &ListedCertificates::new(),
        &ListedCertificates::new(),
        &a_memory(home.path()),
    );

    assert_eq!(rows[0].store, StoreClass::Windows);
    assert_eq!(rows[0].stores, vec![StoreClass::Windows, StoreClass::Card]);
}

#[test]
fn the_installed_store_is_preferred_to_the_browsers() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let der = TestAuthority::root("EIDAS CERTIFICADO PRUEBAS - 99999999R").der();

    let rows = rows_of(
        copies_of(
            &der,
            &[
                &a_firefox_profile(),
                &the_installed_store(&home.path().join("certificates")),
            ],
        ),
        &home.path().join("certificates"),
        &ListedCertificates::new(),
        &ListedCertificates::new(),
        &a_memory(home.path()),
    );

    assert_eq!(rows[0].store, StoreClass::Installed);
}

#[test]
fn the_remembered_copy_is_the_one_behind_the_row_and_marks_it() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let der = TestAuthority::root("EIDAS CERTIFICADO PRUEBAS - 99999999R").der();
    let copies = copies_of(&der, &[&Store::module(CARD), &a_firefox_profile()]);
    remember_the_certificate(&memory, copies[1].reference());
    let listed = ListedCertificates::new();

    let rows = rows_of(
        copies.clone(),
        &home.path().join("certificates"),
        &listed,
        &ListedCertificates::new(),
        &memory,
    );

    assert_eq!(rows.len(), 1);
    assert!(
        rows[0].remembered,
        "la fila esta recordada por su copia de Firefox"
    );
    assert_eq!(rows[0].store, StoreClass::Firefox);
    let chosen = certificate_behind(&copies, &rows[0].id, &listed).expect("la copia");
    assert_eq!(chosen.reference(), copies[1].reference());
}

#[test]
fn a_row_signed_with_a_remembered_browser_copy_still_removes_its_installed_copy() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let installed_dir = home.path().join("certificates");
    let memory = a_memory(home.path());
    let der = TestAuthority::root("EIDAS CERTIFICADO PRUEBAS - 99999999R").der();
    let copies = copies_of(
        &der,
        &[&a_firefox_profile(), &the_installed_store(&installed_dir)],
    );
    remember_the_certificate(&memory, copies[0].reference());
    let listed = ListedCertificates::new();
    let installed_copies = ListedCertificates::new();

    let rows = rows_of(
        copies.clone(),
        &installed_dir,
        &listed,
        &installed_copies,
        &memory,
    );

    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].store,
        StoreClass::Firefox,
        "se sigue firmando con la copia recordada"
    );
    assert!(
        rows[0].stores.contains(&StoreClass::Installed),
        "pero la fila tambien esta instalada"
    );

    remove_installed(
        &RemovalOutcome(Ok(())),
        &FixedPinKeyring("1234"),
        &memory,
        &installed_dir,
        &rows[0].id,
        &listed,
        &installed_copies,
    )
    .expect("la copia instalada deberia poder quitarse aunque la elegida sea de firefox");
}

#[test]
fn unreadable_certificates_are_never_taken_for_copies_of_each_other() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");

    let rows = rows_of(
        vec![
            a_certificate_with_id("ROTO", 0x01, &[0x00]),
            a_certificate_with_id("ROTO", 0x02, &[0x00]),
        ],
        &home.path().join("certificates"),
        &ListedCertificates::new(),
        &ListedCertificates::new(),
        &a_memory(home.path()),
    );

    assert_eq!(rows.len(), 2);
}
