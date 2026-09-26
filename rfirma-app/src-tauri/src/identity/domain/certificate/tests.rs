use super::*;
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::x509::extension::{BasicConstraints, KeyUsage};
use openssl::x509::{X509Name, X509};

use crate::site::domain::local_ca::{generate_key, random_serial};

fn a_certificate_with_extensions(
    label: &str,
    build: impl FnOnce(&mut openssl::x509::X509Builder),
) -> TokenCertificate {
    let key = generate_key().expect("la clave de pruebas deberia generarse");
    let mut name = X509Name::builder().expect("deberia poder construirse un nombre");
    name.append_entry_by_nid(Nid::COMMONNAME, label)
        .expect("el nombre comun deberia entrar");
    let name = name.build();

    let mut builder = X509::builder().expect("deberia poder construirse un certificado");
    builder.set_version(2).expect("la version deberia ponerse");
    builder
        .set_serial_number(&random_serial().expect("el serie deberia generarse"))
        .expect("el serie deberia ponerse");
    builder
        .set_subject_name(&name)
        .expect("el titular deberia ponerse");
    builder
        .set_issuer_name(&name)
        .expect("el emisor deberia ponerse");
    builder.set_pubkey(&key).expect("la clave deberia ponerse");
    builder
        .set_not_before(&openssl::asn1::Asn1Time::days_from_now(0).expect("deberia haber fecha"))
        .expect("el inicio deberia ponerse");
    builder
        .set_not_after(&openssl::asn1::Asn1Time::days_from_now(30).expect("deberia haber fecha"))
        .expect("el fin deberia ponerse");

    build(&mut builder);

    builder
        .sign(&key, MessageDigest::sha256())
        .expect("el certificado de pruebas deberia firmarse");

    let der = builder
        .build()
        .to_der()
        .expect("el certificado deberia poder salir en DER");
    crate::identity::application::tests::a_certificate(label, &der)
}

#[test]
fn a_ca_certificate_cannot_sign_by_content() {
    let certificate = a_certificate_with_extensions("CA", |builder| {
        let extension = BasicConstraints::new()
            .critical()
            .ca()
            .build()
            .expect("basicConstraints deberia construirse");
        builder
            .append_extension(extension)
            .expect("basicConstraints deberia anadirse");
    });

    assert!(certificate.cannot_sign_by_content());
}

#[test]
fn a_key_usage_without_signature_bits_cannot_sign_by_content() {
    let certificate = a_certificate_with_extensions("CIFRADO", |builder| {
        let extension = KeyUsage::new()
            .critical()
            .key_encipherment()
            .build()
            .expect("keyUsage deberia construirse");
        builder
            .append_extension(extension)
            .expect("keyUsage deberia anadirse");
    });

    assert!(certificate.cannot_sign_by_content());
}

#[test]
fn a_certificate_without_key_usage_can_sign_by_content() {
    let certificate = a_certificate_with_extensions("SIN_KEY_USAGE", |_builder| {});

    assert!(!certificate.cannot_sign_by_content());
}

#[test]
fn a_key_usage_with_non_repudiation_can_sign_by_content() {
    let certificate = a_certificate_with_extensions("NO_REPUDIO", |builder| {
        let extension = KeyUsage::new()
            .critical()
            .non_repudiation()
            .build()
            .expect("keyUsage deberia construirse");
        builder
            .append_extension(extension)
            .expect("keyUsage deberia anadirse");
    });

    assert!(!certificate.cannot_sign_by_content());
}

#[test]
fn a_key_usage_with_digital_signature_can_sign_by_content() {
    let certificate = a_certificate_with_extensions("FIRMA_DIGITAL", |builder| {
        let extension = KeyUsage::new()
            .critical()
            .digital_signature()
            .build()
            .expect("keyUsage deberia construirse");
        builder
            .append_extension(extension)
            .expect("keyUsage deberia anadirse");
    });

    assert!(!certificate.cannot_sign_by_content());
}

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
    assert_eq!(certificate.serial_number(), None);
    assert!(!certificate.status().is_usable());
}

#[test]
fn the_serial_number_reads_in_base_ten_like_the_bridge_writes_it() {
    let serial = openssl::bn::BigNum::from_dec_str("12345678901234567890")
        .expect("deberia poder leerse el numero de serie de prueba")
        .to_asn1_integer()
        .expect("deberia poder convertirse a entero ASN.1");
    let certificate = a_certificate_with_extensions("FIRMA", |builder| {
        builder
            .set_serial_number(&serial)
            .expect("el serie deberia ponerse");
    });

    assert_eq!(
        certificate.serial_number(),
        Some("12345678901234567890".to_owned())
    );
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

#[test]
fn a_certificate_past_its_not_after_is_expired_and_not_usable() {
    let certificate = crate::identity::application::tests::an_expired_certificate("CADUCADO");

    assert!(
        matches!(certificate.status(), CertificateStatus::Expired { .. }),
        "{:?}",
        certificate.status()
    );
    assert!(!certificate.status().is_usable());
}
