use super::rfc_name_and_oid;

#[test]
fn rsa_pkcs1_and_ecdsa_with_each_sha_have_their_rfc_name_and_oid() {
    for (jca, name, oid) in [
        (
            "SHA256withRSA",
            "sha256WithRSAEncryption",
            "1.2.840.113549.1.1.11",
        ),
        (
            "SHA384withRSA",
            "sha384WithRSAEncryption",
            "1.2.840.113549.1.1.12",
        ),
        (
            "SHA512withRSA",
            "sha512WithRSAEncryption",
            "1.2.840.113549.1.1.13",
        ),
        (
            "SHA256withECDSA",
            "ecdsa-with-SHA256",
            "1.2.840.10045.4.3.2",
        ),
        (
            "SHA384withECDSA",
            "ecdsa-with-SHA384",
            "1.2.840.10045.4.3.3",
        ),
        (
            "SHA512withECDSA",
            "ecdsa-with-SHA512",
            "1.2.840.10045.4.3.4",
        ),
    ] {
        assert_eq!(rfc_name_and_oid(jca), Some((name, oid)), "{jca}");
    }
}

#[test]
fn rsa_pss_names_of_jca_give_rsassa_pss() {
    for jca in [
        "RSASSA-PSS",
        "SHA256withRSAandMGF1",
        "SHA384withRSAandMGF1",
        "SHA512withRSAandMGF1",
    ] {
        assert_eq!(
            rfc_name_and_oid(jca),
            Some(("rsassaPss", "1.2.840.113549.1.1.10")),
            "{jca}"
        );
    }
}

#[test]
fn an_unknown_name_has_no_translation() {
    assert_eq!(rfc_name_and_oid("MD5withRSA"), None);
    assert_eq!(rfc_name_and_oid(""), None);
}
