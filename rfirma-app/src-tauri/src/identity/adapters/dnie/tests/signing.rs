use rdnie_emulator::EmulatedDnie;
use rsa::{Pkcs1v15Sign, RsaPublicKey};
use sha2::{Digest, Sha256, Sha384, Sha512};

use super::in_the_reader::{the_pin, the_signing_certificate, InTheReader};
use super::{the_dnie, the_two_versions};
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::error::Situation;
use crate::identity::domain::secret::PinWarning;
use crate::identity::ports::Token;

const SIGNED: &[u8] = b"los atributos firmados de un documento";

fn verifies(key: &RsaPublicKey, algorithm: SignatureAlgorithm, signature: &[u8]) -> bool {
    let (scheme, digest) = match algorithm {
        SignatureAlgorithm::Sha256Rsa => (
            Pkcs1v15Sign::new::<Sha256>(),
            Sha256::digest(SIGNED).to_vec(),
        ),
        SignatureAlgorithm::Sha384Rsa => (
            Pkcs1v15Sign::new::<Sha384>(),
            Sha384::digest(SIGNED).to_vec(),
        ),
        SignatureAlgorithm::Sha512Rsa => (
            Pkcs1v15Sign::new::<Sha512>(),
            Sha512::digest(SIGNED).to_vec(),
        ),
        other => panic!("el DNIe no firma con {other:?}"),
    };
    key.verify(scheme, &digest, signature).is_ok()
}

#[test]
fn signs_with_sha256_sha384_and_sha512_and_the_signature_verifies_with_the_card_key() {
    for (version, card) in the_two_versions() {
        for algorithm in [
            SignatureAlgorithm::Sha256Rsa,
            SignatureAlgorithm::Sha384Rsa,
            SignatureAlgorithm::Sha512Rsa,
        ] {
            let reader = InTheReader::new(card());
            let key = reader.card().signing_public_key();

            let signature = reader
                .token()
                .sign_with_secret(&the_signing_certificate(), &the_pin(), algorithm, SIGNED)
                .unwrap_or_else(|error| panic!("{version} {algorithm:?}: {error}"));

            assert!(
                verifies(&key, algorithm, &signature),
                "{version} {algorithm:?}"
            );
        }
    }
}

#[test]
fn the_dnie_signs_its_copy_through_rdnie_with_its_attempts_left() {
    let reader = InTheReader::new(EmulatedDnie::dnie_4().with_pin_retries(2));
    let key = reader.card().signing_public_key();
    let token = reader.token();

    let warning = token.pin_warning(&the_signing_certificate());
    let signature = token
        .sign_with_secret(
            &the_signing_certificate(),
            &the_pin(),
            SignatureAlgorithm::Sha256Rsa,
            SIGNED,
        )
        .expect("el DNIe deberia firmar");

    assert_eq!(warning, Ok(PinWarning::AttemptsLeft(2)));
    assert!(verifies(&key, SignatureAlgorithm::Sha256Rsa, &signature));
}

#[test]
fn the_pin_warning_gives_the_exact_attempts_left_without_spending_any() {
    for (version, card) in the_two_versions() {
        for left in [3, 2, 1, 0] {
            let reader = InTheReader::new(card().with_pin_retries(left));
            let dnie = reader.token();

            let warning = dnie.pin_warning(&the_signing_certificate());

            assert_eq!(
                warning,
                Ok(PinWarning::AttemptsLeft(u32::from(left))),
                "{version}"
            );
            assert_eq!(reader.pins_sent(), 0, "{version}");
            assert_eq!(
                dnie.pin_warning(&the_signing_certificate()),
                Ok(PinWarning::AttemptsLeft(u32::from(left))),
                "{version}"
            );
        }
    }
}

#[test]
fn offers_rsa_pkcs1_with_sha256_sha384_and_sha512_and_nothing_else() {
    let dnie = the_dnie(EmulatedDnie::dnie_3);
    let signing = the_signing_certificate();

    let offered: Vec<SignatureAlgorithm> = SignatureAlgorithm::ALL
        .into_iter()
        .filter(|algorithm| dnie.offers(&signing, *algorithm).is_ok())
        .collect();

    assert_eq!(
        offered,
        vec![
            SignatureAlgorithm::Sha256Rsa,
            SignatureAlgorithm::Sha384Rsa,
            SignatureAlgorithm::Sha512Rsa
        ]
    );
    for refused in [
        SignatureAlgorithm::Sha1Rsa,
        SignatureAlgorithm::Sha256RsaPss,
    ] {
        assert_eq!(
            dnie.offers(&signing, refused)
                .expect_err("el DNIe no ofrece ese algoritmo")
                .situation(),
            Situation::MechanismNotOffered
        );
    }
}
