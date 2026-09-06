use super::{SessionSeal, TokenSignature, ALGORITHM};
use std::collections::BTreeSet;

const BORDER: &str = include_str!("../../adapters/ffi.rs");

fn identifiers(source: &str) -> BTreeSet<&str> {
    source
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
        .collect()
}

fn entry_points() -> BTreeSet<String> {
    BORDER
        .match_indices("autofirma_")
        .map(|(start, _)| {
            BORDER[start..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect()
        })
        .collect()
}

#[test]
fn java_has_no_entry_point_for_the_signing_phase() {
    let expected: BTreeSet<String> = [
        "autofirma_expand_extra_params",
        "autofirma_filter_certificates",
        "autofirma_free_string",
        "autofirma_pades_postsign",
        "autofirma_pades_presign",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();

    assert_eq!(
        entry_points(),
        expected,
        "la frontera con Java ha cambiado de puntos de entrada: si uno de \
         ellos firma, la clave privada ha entrado en el isolate (ADR-0001)"
    );
}

#[test]
fn the_pin_has_no_way_across_the_border() {
    let words = identifiers(BORDER);
    for forbidden in ["pin", "private_key", "AuthPin", "cryptoki"] {
        assert!(
            !words.contains(forbidden),
            "«{forbidden}» aparece en la frontera FFI: la fase 2 se estaría \
             delegando a Java, contra el ADR-0001"
        );
    }
}

#[test]
fn only_the_pkcs11_module_talks_to_the_token() {
    let cycle = include_str!("../cycle.rs");

    assert!(cycle.contains("pkcs11::sign(&self.certificate, pin"));
    assert!(!identifiers(cycle).contains("sign_on_bridge"));
    assert_eq!(cycle.matches("bridge.").count(), 2);
}

#[test]
fn the_algorithm_matches_the_pkcs11_mechanism() {
    let token_side = include_str!("../../../identity/adapters/pkcs11/mod.rs");

    assert_eq!(ALGORITHM, "SHA256withRSA");
    assert!(token_side.contains("Mechanism::Sha256RsaPkcs"));
}

#[test]
fn the_signature_travels_to_the_postsign_in_base64() {
    let signature = TokenSignature(vec![0x30, 0x82, 0x01, 0x00]);

    assert_eq!(signature.raw(), [0x30, 0x82, 0x01, 0x00]);
    assert_eq!(signature.to_pkcs1_base64(), "MIIBAA==");
}

#[test]
fn a_seal_that_came_back_changed_is_refused_before_anything_else() {
    let issued = SessionSeal::from_bridge("el sello de la prefirma");
    let tampered = SessionSeal::from_bridge("el sello de la prefirma.");

    assert!(issued.verify_unchanged(&tampered).is_err());
    assert!(issued.verify_unchanged(&issued.clone()).is_ok());
}

#[test]
fn the_postsign_compares_the_seal_before_crossing_the_border() {
    let cycle = include_str!("../cycle.rs");
    let body = cycle
        .split_once("pub fn postsign(")
        .expect("la postfirma sigue aquí")
        .1;
    let check = body
        .find("verify_unchanged")
        .expect("la postfirma comprueba el sello");
    let crossing = body.find("bridge.postsign").expect("y luego cruza");

    assert!(
        check < crossing,
        "el sello se comprueba después de cruzar: el PDF ya saldría mal"
    );
}
