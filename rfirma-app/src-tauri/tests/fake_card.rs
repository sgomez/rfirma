//! Pruebas de integración del backend contra el módulo PKCS#11 falso, como un almacén de clase tarjeta (ADR-0014).

#![cfg(not(windows))]

use std::path::Path;

use fake_pkcs11::FakeCard;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::adapters::pkcs11::stores::{candidate_modules_under, CANDIDATE_MODULES};
use rfirma_lib::identity::domain::store::{Store, StoreClass};

const SIGNING_CERTIFICATE: &str = "CertFirmaDigital";

#[test]
fn the_fake_module_is_a_card_store_that_lists_without_asking_for_the_pin() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    let store = Store::module(card.module());
    assert_eq!(store.class(), StoreClass::Card);

    let listed = pkcs11::list_certificates_across(&[store]).expect("listar no debería fallar");

    assert!(
        listed
            .iter()
            .any(|certificate| certificate.reference().label() == SIGNING_CERTIFICATE),
        "el certificado de firma tenía que salir sin sesión"
    );
    assert!(
        card.calls_to("C_Login").is_empty(),
        "listar pidió el secreto a la tarjeta: {:?}",
        card.calls()
    );
}

#[test]
fn the_fake_module_is_not_among_the_modules_rfirma_looks_for() {
    let candidates: Vec<String> = candidate_modules_under(Path::new("/usr/lib"))
        .iter()
        .map(|path| path.display().to_string())
        .chain(CANDIDATE_MODULES.iter().map(|path| (*path).to_owned()))
        .collect();

    assert!(
        candidates.iter().all(|path| !path.contains("fake")),
        "{candidates:?}"
    );
}
