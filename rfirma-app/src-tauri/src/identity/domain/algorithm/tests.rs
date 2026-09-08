use cryptoki::mechanism::MechanismType;

use super::{KeyKind, SignatureAlgorithm};

/// Nombre del original, mecanismo `cryptoki` y clase de clave, para todos los que acepta.
const TABLE: [(&str, MechanismType, KeyKind); 9] = [
    (
        "SHA256withRSA",
        MechanismType::SHA256_RSA_PKCS,
        KeyKind::Rsa,
    ),
    (
        "SHA384withRSA",
        MechanismType::SHA384_RSA_PKCS,
        KeyKind::Rsa,
    ),
    (
        "SHA512withRSA",
        MechanismType::SHA512_RSA_PKCS,
        KeyKind::Rsa,
    ),
    (
        "SHA256withRSAandMGF1",
        MechanismType::SHA256_RSA_PKCS_PSS,
        KeyKind::Rsa,
    ),
    (
        "SHA384withRSAandMGF1",
        MechanismType::SHA384_RSA_PKCS_PSS,
        KeyKind::Rsa,
    ),
    (
        "SHA512withRSAandMGF1",
        MechanismType::SHA512_RSA_PKCS_PSS,
        KeyKind::Rsa,
    ),
    ("SHA256withECDSA", MechanismType::ECDSA_SHA256, KeyKind::Ec),
    ("SHA384withECDSA", MechanismType::ECDSA_SHA384, KeyKind::Ec),
    ("SHA512withECDSA", MechanismType::ECDSA_SHA512, KeyKind::Ec),
];

#[test]
fn every_accepted_name_fixes_its_mechanism_and_its_key_kind() {
    for (name, mechanism, key_kind) in TABLE {
        let algorithm = SignatureAlgorithm::from_name(name)
            .unwrap_or_else(|| panic!("{name} tendria que estar aceptado"));

        assert_eq!(algorithm.name(), name);
        assert_eq!(algorithm.mechanism_type(), mechanism, "{name}");
        assert_eq!(algorithm.key_kind(), key_kind, "{name}");
    }
}

#[test]
fn the_table_covers_every_algorithm_and_no_two_share_a_mechanism() {
    assert_eq!(SignatureAlgorithm::ALL.len(), TABLE.len());

    for (position, algorithm) in SignatureAlgorithm::ALL.into_iter().enumerate() {
        let sharing = SignatureAlgorithm::ALL[position + 1..]
            .iter()
            .find(|other| other.mechanism_type() == algorithm.mechanism_type());

        assert!(sharing.is_none(), "{} comparte mecanismo", algorithm.name());
    }
}

#[test]
fn the_composed_mechanism_is_the_one_the_name_promises() {
    for algorithm in SignatureAlgorithm::ALL {
        assert_eq!(
            algorithm.mechanism().mechanism_type(),
            algorithm.mechanism_type(),
            "{}",
            algorithm.name()
        );
    }
}

#[test]
fn a_name_in_any_case_and_with_spaces_around_is_the_same_algorithm() {
    assert_eq!(
        SignatureAlgorithm::from_name("  sha256withrsa  "),
        Some(SignatureAlgorithm::Sha256Rsa)
    );
}

#[test]
fn a_name_the_original_does_not_accept_is_no_algorithm() {
    assert_eq!(SignatureAlgorithm::from_name("SHA1withRSA"), None);
    assert_eq!(SignatureAlgorithm::from_name("SHA256"), None);
}
