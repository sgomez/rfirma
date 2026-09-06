use super::*;

#[test]
fn a_reference_carries_the_four_coordinates_and_nothing_else() {
    let reference =
        CertificateRef::new("/usr/lib/x.so", "rfirma-test", "ETIQUETA", vec![0x2a, 0x01]);

    assert_eq!(reference.module(), Path::new("/usr/lib/x.so"));
    assert_eq!(reference.token_label(), "rfirma-test");
    assert_eq!(reference.label(), "ETIQUETA");
    assert_eq!(reference.cka_id(), Some([0x2a, 0x01].as_slice()));
}

#[test]
fn a_reference_remembered_before_the_cka_id_existed_still_reads() {
    let written = r#"{
        "module": "/usr/lib/x.so",
        "token_label": "rfirma-test",
        "label": "ETIQUETA"
    }"#;

    let reference: CertificateRef =
        serde_json::from_str(written).expect("una referencia antigua tiene que leerse");

    assert_eq!(reference.label(), "ETIQUETA");
    assert_eq!(reference.cka_id(), None);
}

#[test]
fn a_reference_round_trips_through_the_state_file_with_its_cka_id() {
    let reference = CertificateRef::new("/usr/lib/x.so", "rfirma-test", "ETIQUETA", vec![0x05]);

    let written = serde_json::to_string(&reference).expect("deberia serializarse");
    let read: CertificateRef = serde_json::from_str(&written).expect("deberia leerse");

    assert_eq!(read, reference);
    assert_eq!(read.cka_id(), Some([0x05].as_slice()));
}

#[test]
fn a_der_that_is_not_a_certificate_is_unreadable_rather_than_a_panic() {
    let certificate = TokenCertificate::new(
        CertificateRef::new("/usr/lib/x.so", "rfirma-test", "BASURA", vec![0x01]),
        vec![0x00, 0x01, 0x02],
    );

    assert!(matches!(
        certificate.status(),
        CertificateStatus::Unreadable { .. }
    ));
    assert_eq!(certificate.subject(), None);
    assert_eq!(certificate.issuer(), None);
    assert!(!certificate.status().is_usable());
}

#[test]
fn a_remembered_reference_recognises_the_one_that_came_out_of_the_token() {
    let remembered = CertificateRef::new("/usr/lib/x.so", "rfirma-test", "FIRMA", vec![0x01]);

    assert!(remembered.is_the_same_as(&CertificateRef::new(
        "/usr/lib/x.so",
        "rfirma-test",
        "FIRMA",
        vec![0x01]
    )));
    assert!(!remembered.is_the_same_as(&CertificateRef::new(
        "/usr/lib/x.so",
        "rfirma-test",
        "FIRMA",
        vec![0x02]
    )));
    assert!(!remembered.is_the_same_as(&CertificateRef::new(
        "/usr/lib/x.so",
        "otro-token",
        "FIRMA",
        vec![0x01]
    )));
    assert!(!remembered.is_the_same_as(&CertificateRef::new(
        "/usr/lib/otro.so",
        "rfirma-test",
        "FIRMA",
        vec![0x01]
    )));
}

#[test]
fn a_reference_remembered_by_an_older_version_still_finds_its_certificate() {
    let written = r#"{
        "module": "/usr/lib/libsoftokn3.so",
        "token_label": "NSS Certificate DB",
        "label": "FIRMA"
    }"#;
    let remembered: CertificateRef =
        serde_json::from_str(written).expect("una referencia antigua tiene que leerse");

    let listed = CertificateRef::new(
        Store::with_init_args(
            "/usr/lib/libsoftokn3.so",
            Some("configdir='/home/quien/.mozilla/firefox/abc'".to_owned()),
        ),
        "NSS Certificate DB",
        "FIRMA",
        vec![0x01],
    );

    assert!(remembered.is_the_same_as(&listed));
}

#[test]
fn two_firefox_profiles_are_not_the_same_certificate() {
    let one = CertificateRef::new(
        Store::with_init_args(
            "/usr/lib/libsoftokn3.so",
            Some("configdir='/uno'".to_owned()),
        ),
        "NSS Certificate DB",
        "FIRMA",
        vec![0x01],
    );
    let other = CertificateRef::new(
        Store::with_init_args(
            "/usr/lib/libsoftokn3.so",
            Some("configdir='/otro'".to_owned()),
        ),
        "NSS Certificate DB",
        "FIRMA",
        vec![0x01],
    );

    assert!(!one.is_the_same_as(&other));
}

#[test]
fn a_revocation_is_not_a_token_failure() {
    let status = CertificateStatus::Revoked {
        reason: "superseded".to_owned(),
    };

    assert!(!status.is_usable());
}
