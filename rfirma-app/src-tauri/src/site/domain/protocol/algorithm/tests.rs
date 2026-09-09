use super::*;

#[test]
fn every_name_the_published_client_sends_names_its_digest() {
    for (name, expected) in [
        ("SHA256", AskedAlgorithm::Sha256),
        ("SHA384", AskedAlgorithm::Sha384),
        ("SHA512", AskedAlgorithm::Sha512),
        ("SHA-256", AskedAlgorithm::Sha256),
        ("SHA-384", AskedAlgorithm::Sha384),
        ("SHA-512", AskedAlgorithm::Sha512),
        ("sha256", AskedAlgorithm::Sha256),
        ("sha-256", AskedAlgorithm::Sha256),
        ("SHA256withRSA", AskedAlgorithm::Sha256),
        ("SHA-256withRSA", AskedAlgorithm::Sha256),
        ("SHA384withRSA", AskedAlgorithm::Sha384),
        ("SHA-384withRSA", AskedAlgorithm::Sha384),
        ("SHA512withRSA", AskedAlgorithm::Sha512),
        ("SHA-512withRSA", AskedAlgorithm::Sha512),
        ("SHA256withECDSA", AskedAlgorithm::Sha256),
        ("SHA-256withECDSA", AskedAlgorithm::Sha256),
        ("SHA384withECDSA", AskedAlgorithm::Sha384),
        ("SHA-384withECDSA", AskedAlgorithm::Sha384),
        ("SHA512withECDSA", AskedAlgorithm::Sha512),
        ("SHA-512withECDSA", AskedAlgorithm::Sha512),
        ("SHA256withDSA", AskedAlgorithm::Sha256),
        ("2.16.840.1.101.3.4.2.1", AskedAlgorithm::Sha256),
        ("2.16.840.1.101.3.4.2.2", AskedAlgorithm::Sha384),
        ("2.16.840.1.101.3.4.2.3", AskedAlgorithm::Sha512),
        (
            "http://www.w3.org/2001/04/xmlenc#sha256",
            AskedAlgorithm::Sha256,
        ),
        (
            "http://www.w3.org/2001/04/xmlenc#sha512",
            AskedAlgorithm::Sha512,
        ),
        (
            "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256",
            AskedAlgorithm::Sha256,
        ),
        (
            "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha384",
            AskedAlgorithm::Sha384,
        ),
        (
            "http://www.w3.org/2001/04/xmldsig-more#rsa-sha512",
            AskedAlgorithm::Sha512,
        ),
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
    assert_eq!(
        AskedAlgorithm::named(" sha-256 "),
        Some(AskedAlgorithm::Sha256)
    );
}

#[test]
fn what_rfirma_does_not_sign_names_no_digest() {
    for name in [
        "SHA",
        "SHA1",
        "SHA-1",
        "sha1",
        "sha-1",
        "SHA1withRSA",
        "SHA-1withRSA",
        "SHA1withECDSA",
        "1.3.14.3.2.26",
        "http://www.w3.org/2000/09/xmldsig#sha1",
        "http://www.w3.org/2001/04/xmldsig-more#rsa-sha1",
        "MD5",
        "MD5withRSA",
        "RIPEMD160",
        "RIPEMD-160",
        "http://www.w3.org/2001/04/xmlenc#ripemd160",
        "",
        "   ",
        "desconocido",
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
