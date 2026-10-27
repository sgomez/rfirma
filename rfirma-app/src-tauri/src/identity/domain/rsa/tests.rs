use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::{Padding, Rsa};
use openssl::sign::Verifier;

use super::{digest_info, signs_a_digest_info};
use crate::identity::domain::algorithm::SignatureAlgorithm;

#[test]
fn a_raw_signature_over_the_digest_info_verifies_as_the_composed_one() {
    let rsa = Rsa::generate(2048).expect("clave RSA de prueba");
    let key = PKey::from_rsa(rsa.clone()).expect("clave de prueba");
    let data = b"atributos firmados";

    for (algorithm, digest) in [
        (SignatureAlgorithm::Sha1Rsa, MessageDigest::sha1()),
        (SignatureAlgorithm::Sha256Rsa, MessageDigest::sha256()),
        (SignatureAlgorithm::Sha384Rsa, MessageDigest::sha384()),
        (SignatureAlgorithm::Sha512Rsa, MessageDigest::sha512()),
    ] {
        let block = digest_info(algorithm, data).expect("DigestInfo de PKCS#1 v1.5");
        let mut signature = vec![0; rsa.size() as usize];
        rsa.private_encrypt(&block, &mut signature, Padding::PKCS1)
            .expect("firma cruda");

        let mut verifier = Verifier::new(digest, &key).expect("verificador");
        verifier.update(data).expect("datos");
        assert!(
            verifier.verify(&signature).expect("verificación"),
            "{}",
            algorithm.name()
        );
    }
}

#[test]
fn neither_pss_nor_ecdsa_are_signed_over_a_digest_info() {
    for algorithm in [
        SignatureAlgorithm::Sha256RsaPss,
        SignatureAlgorithm::Sha256Ecdsa,
    ] {
        assert!(!signs_a_digest_info(algorithm), "{}", algorithm.name());
        assert!(digest_info(algorithm, b"").is_err(), "{}", algorithm.name());
    }
}
