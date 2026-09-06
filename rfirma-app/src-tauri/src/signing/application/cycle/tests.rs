use super::ALGORITHM;
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
fn the_algorithm_matches_the_pkcs11_mechanism() {
    let token_side = include_str!("../../../identity/adapters/pkcs11/mod.rs");

    assert_eq!(ALGORITHM, "SHA256withRSA");
    assert!(token_side.contains("Mechanism::Sha256RsaPkcs"));
}
