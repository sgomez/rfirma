use super::issuers_of;
use crate::identity::application::tests::TestAuthority;

#[test]
fn a_certificate_takes_the_issuer_that_is_in_its_store() {
    let root = TestAuthority::root("Raiz de pruebas");
    let authority = root.issues("AC de pruebas");
    let signer = authority.issues("Firmante de pruebas");

    let issuers = issuers_of(
        &signer.as_certificate("firmante"),
        &[
            signer.as_certificate("firmante"),
            authority.as_certificate("AC"),
        ],
    );

    assert_eq!(issuers, vec![authority.der()]);
}

#[test]
fn a_certificate_whose_issuer_is_not_in_the_store_goes_alone() {
    let root = TestAuthority::root("Raiz de pruebas");
    let authority = root.issues("AC de pruebas");
    let signer = authority.issues("Firmante de pruebas");

    let issuers = issuers_of(
        &signer.as_certificate("firmante"),
        &[signer.as_certificate("firmante")],
    );

    assert!(issuers.is_empty());
}

#[test]
fn the_root_stays_out_because_it_is_the_anchor_the_verifier_puts() {
    let root = TestAuthority::root("Raiz de pruebas");
    let authority = root.issues("AC de pruebas");
    let signer = authority.issues("Firmante de pruebas");

    let issuers = issuers_of(
        &signer.as_certificate("firmante"),
        &[authority.as_certificate("AC"), root.as_certificate("raiz")],
    );

    assert_eq!(issuers, vec![authority.der()]);
}

#[test]
fn a_self_issued_certificate_adds_nothing_to_itself() {
    let root = TestAuthority::root("Raiz de pruebas");

    let issuers = issuers_of(&root.as_certificate("raiz"), &[root.as_certificate("raiz")]);

    assert!(issuers.is_empty());
}

#[test]
fn two_issuers_come_back_from_the_nearest_to_the_farthest() {
    let root = TestAuthority::root("Raiz de pruebas");
    let upper = root.issues("AC superior de pruebas");
    let lower = upper.issues("AC inferior de pruebas");
    let signer = lower.issues("Firmante de pruebas");

    let issuers = issuers_of(
        &signer.as_certificate("firmante"),
        &[
            root.as_certificate("raiz"),
            lower.as_certificate("AC inferior"),
            upper.as_certificate("AC superior"),
        ],
    );

    assert_eq!(issuers, vec![lower.der(), upper.der()]);
}
