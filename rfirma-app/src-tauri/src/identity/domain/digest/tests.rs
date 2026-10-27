use super::digest;
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
