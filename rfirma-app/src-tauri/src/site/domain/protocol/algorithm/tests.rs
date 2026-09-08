use super::*;

#[test]
fn every_name_the_published_client_sends_names_its_digest() {
    for (name, expected) in [
        ("SHA256", AskedAlgorithm::Sha256),
        ("SHA384", AskedAlgorithm::Sha384),
        ("SHA512", AskedAlgorithm::Sha512),
        ("SHA256withRSA", AskedAlgorithm::Sha256),
        ("SHA384withRSA", AskedAlgorithm::Sha384),
        ("SHA512withRSA", AskedAlgorithm::Sha512),
        ("SHA256withECDSA", AskedAlgorithm::Sha256),
        ("SHA384withECDSA", AskedAlgorithm::Sha384),
        ("SHA512withECDSA", AskedAlgorithm::Sha512),
    ] {
        assert_eq!(AskedAlgorithm::named(name), Some(expected), "{name}");
    }
}

#[test]
fn the_name_is_read_without_case_nor_spaces() {
    assert_eq!(
        AskedAlgorithm::named(" sha512withrsa "),
        Some(AskedAlgorithm::Sha512)
    );
}

#[test]
fn what_rfirma_does_not_sign_names_no_digest() {
    for name in [
        "SHA1",
        "SHA1withRSA",
        "SHA1withECDSA",
        "SHA256withDSA",
        "MD5withRSA",
        "",
    ] {
        assert_eq!(AskedAlgorithm::named(name), None, "{name}");
    }
}

#[test]
fn the_digest_is_named_as_the_bridge_reads_it() {
    assert_eq!(AskedAlgorithm::Sha256.name(), "SHA256");
    assert_eq!(AskedAlgorithm::Sha384.name(), "SHA384");
    assert_eq!(AskedAlgorithm::Sha512.name(), "SHA512");
}
