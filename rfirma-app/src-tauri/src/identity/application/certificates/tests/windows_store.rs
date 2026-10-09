use std::path::Path;

use super::super::{
    certificate_behind, certificates_with_their_chains, listed_rows, ListedCertificates,
};
use super::StoresWith;
use crate::identity::application::readers::LastListing;
use crate::identity::application::tests::{a_dnie_der, NoMemory, TestAuthority};
use crate::identity::domain::certificate::{CertificateRef, ListedCertificate, TokenCertificate};
use crate::identity::domain::store::{Store, StoreClass};

const CARD_KSP: &str = "Microsoft Smart Card Key Storage Provider";
const SOFTWARE_KSP: &str = "Microsoft Software Key Storage Provider";

fn the_windows_store() -> Store {
    Store::module("cng:CurrentUser/MY")
}

/// Un certificado de `CurrentUser\MY` con su clave en ese proveedor.
fn in_windows_with_its_key_in(provider: &str, label: &str, der: &[u8]) -> TokenCertificate {
    TokenCertificate::new(
        CertificateRef::new(
            the_windows_store(),
            "CurrentUser\\MY",
            label,
            label.as_bytes().to_vec(),
        )
        .with_key_provider(provider),
        der.to_vec(),
    )
}

fn in_firefox(der: &[u8]) -> TokenCertificate {
    TokenCertificate::new(
        CertificateRef::new(
            Store::nss(
                "/usr/lib/libsoftokn3.so",
                Path::new("/home/ada/.mozilla/firefox/perfil"),
            ),
            "NSS Certificate DB",
            "COPIA-FIREFOX",
            vec![0x01],
        ),
        der.to_vec(),
    )
}

/// Las filas del escritorio que salen de esos almacenes, y el listado tras sus asas.
fn listed_on_the_desktop(
    found: &[TokenCertificate],
    stores: &[Store],
) -> (Vec<ListedCertificate>, ListedCertificates) {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let listed = ListedCertificates::new();
    let rows = listed_rows(
        &StoresWith::holding(found.to_vec(), found.to_vec()),
        stores,
        &home.path().join("certificates"),
        &listed,
        &ListedCertificates::new(),
        &NoMemory,
        &LastListing::default(),
    )
    .expect("los almacenes deberian listarse");
    (rows, listed)
}

fn the_row_of<'a>(rows: &'a [ListedCertificate], label: &str) -> &'a ListedCertificate {
    rows.iter()
        .find(|row| row.label == label)
        .unwrap_or_else(|| panic!("deberia haber una fila de {label}"))
}

#[test]
fn a_windows_certificate_with_its_key_on_a_card_is_a_card_and_one_in_software_stays_in_windows() {
    let found = [
        in_windows_with_its_key_in(CARD_KSP, "DNIE-FIRMA", &a_dnie_der("DNIE FIRMA", true)),
        in_windows_with_its_key_in(
            CARD_KSP,
            "OTRA-TARJETA",
            &TestAuthority::root("OTRA TARJETA").der(),
        ),
        in_windows_with_its_key_in(
            SOFTWARE_KSP,
            "EN-SOFTWARE",
            &TestAuthority::root("EN SOFTWARE").der(),
        ),
    ];

    let (rows, _) = listed_on_the_desktop(&found, &[the_windows_store()]);

    let dnie = the_row_of(&rows, "DNIE-FIRMA");
    assert_eq!((dnie.store, dnie.from_a_dnie), (StoreClass::Card, true));
    let card = the_row_of(&rows, "OTRA-TARJETA");
    assert_eq!((card.store, card.from_a_dnie), (StoreClass::Card, false));
    let software = the_row_of(&rows, "EN-SOFTWARE");
    assert_eq!(software.store, StoreClass::Windows);
}

#[test]
fn a_windows_certificate_whose_key_provider_is_unknown_stays_in_windows() {
    let found = [TokenCertificate::new(
        CertificateRef::new(
            the_windows_store(),
            "CurrentUser\\MY",
            "SIN-PROVEEDOR",
            vec![0x01],
        ),
        TestAuthority::root("SIN PROVEEDOR").der(),
    )];

    let (rows, _) = listed_on_the_desktop(&found, &[the_windows_store()]);

    assert_eq!(rows[0].store, StoreClass::Windows);
}

#[test]
fn of_two_copies_the_one_with_its_key_on_the_card_signs() {
    let der = a_dnie_der("DNIE FIRMA", true);
    let firefox = Store::nss(
        "/usr/lib/libsoftokn3.so",
        Path::new("/home/ada/.mozilla/firefox/perfil"),
    );
    let found = [
        in_firefox(&der),
        in_windows_with_its_key_in(CARD_KSP, "DNIE-FIRMA", &der),
    ];

    let (rows, listed) = listed_on_the_desktop(&found, &[firefox, the_windows_store()]);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].store, StoreClass::Card);
    assert_eq!(rows[0].stores, vec![StoreClass::Card, StoreClass::Firefox]);
    let chosen = certificate_behind(&found, &rows[0].id, &listed).expect("la copia");
    assert_eq!(chosen.reference(), found[1].reference());
}

#[test]
fn on_the_desktop_the_dnie_authentication_certificate_through_windows_is_hidden() {
    let found = [
        in_windows_with_its_key_in(CARD_KSP, "DNIE-FIRMA", &a_dnie_der("DNIE FIRMA", true)),
        in_windows_with_its_key_in(
            CARD_KSP,
            "DNIE-AUTENTICACION",
            &a_dnie_der("DNIE AUTENTICACION", false),
        ),
    ];

    let (rows, _) = listed_on_the_desktop(&found, &[the_windows_store()]);

    let labels: Vec<&str> = rows.iter().map(|row| row.label.as_str()).collect();
    assert_eq!(labels, vec!["DNIE-FIRMA"]);
}

#[test]
fn a_site_is_offered_both_dnie_certificates_through_windows() {
    let found = vec![
        in_windows_with_its_key_in(CARD_KSP, "DNIE-FIRMA", &a_dnie_der("DNIE FIRMA", true)),
        in_windows_with_its_key_in(
            CARD_KSP,
            "DNIE-AUTENTICACION",
            &a_dnie_der("DNIE AUTENTICACION", false),
        ),
    ];

    let offered: Vec<String> = certificates_with_their_chains(
        &StoresWith::holding(found.clone(), found),
        &[the_windows_store()],
    )
    .expect("el almacen deberia listarse")
    .iter()
    .map(|certificate| certificate.reference().label().to_owned())
    .collect();

    assert_eq!(offered, vec!["DNIE-FIRMA", "DNIE-AUTENTICACION"]);
}
