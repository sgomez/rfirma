use super::{der_encoded, digest};
use crate::identity::domain::algorithm::SignatureAlgorithm;

/// SHA-256 de la cadena vacía.
const EMPTY_SHA256: [u8; 32] = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
];

#[test]
fn the_digest_is_the_one_the_algorithm_names() {
    assert_eq!(
        digest(SignatureAlgorithm::Sha256Ecdsa, b"").expect("SHA-256 de la cadena vacía"),
        EMPTY_SHA256
    );
    assert_eq!(
        digest(SignatureAlgorithm::Sha384Ecdsa, b"")
            .expect("SHA-384 de la cadena vacía")
            .len(),
        48
    );
    assert_eq!(
        digest(SignatureAlgorithm::Sha512Ecdsa, b"")
            .expect("SHA-512 de la cadena vacía")
            .len(),
        64
    );
}

#[test]
fn concatenated_r_and_s_come_back_as_a_sequence_of_two_integers() {
    assert_eq!(
        der_encoded(&[0x01, 0x02, 0x03, 0x04]).expect("r||s de cuatro bytes se reempaqueta"),
        vec![0x30, 0x08, 0x02, 0x02, 0x01, 0x02, 0x02, 0x02, 0x03, 0x04]
    );
}

#[test]
fn an_r_with_the_high_bit_set_keeps_the_zero_that_makes_it_positive() {
    assert_eq!(
        der_encoded(&[0x80, 0x01, 0x00, 0x02]).expect("r||s con el bit alto puesto"),
        vec![0x30, 0x08, 0x02, 0x03, 0x00, 0x80, 0x01, 0x02, 0x01, 0x02]
    );
}

#[test]
fn the_sixty_four_bytes_of_a_p256_token_become_a_sequence_of_two_thirty_two_byte_integers() {
    let der = der_encoded(&[0x7f; 64]).expect("r||s de una curva P-256 se reempaqueta");

    assert_eq!(der[0], 0x30);
    assert_eq!(der.len(), 2 + 2 * (2 + 32));
}

#[test]
fn a_signature_with_an_odd_number_of_bytes_is_refused() {
    let refusal = der_encoded(&[0x01, 0x02, 0x03]).expect_err("r||s impar no es r||s");

    assert!(
        refusal.detail().contains("3 bytes"),
        "el detalle dice cuántos bytes ha devuelto el token: {refusal}"
    );
}

#[test]
fn an_empty_signature_is_refused() {
    assert!(der_encoded(&[]).is_err());
}
