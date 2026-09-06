use super::*;

fn text_of(ca: &LocalCa) -> String {
    String::from_utf8(ca.certificate().to_text().expect("deberia imprimirse"))
        .expect("deberia ser UTF-8")
}

#[test]
fn the_local_ca_can_only_vouch_for_the_loopback() {
    let text = text_of(&LocalCa::generate().expect("deberia generarse"));

    assert!(
        text.contains("X509v3 Name Constraints: critical"),
        "sin la restriccion la CA local podria afirmar cualquier sitio web:\n{text}"
    );
    assert!(text.contains("DNS:localhost"), "{text}");
    assert!(text.contains("IP:127.0.0.1/255.255.255.255"), "{text}");
    assert!(
        text.contains("IP:0:0:0:0:0:0:0:1/FFFF:FFFF:FFFF:FFFF:FFFF:FFFF:FFFF:FFFF"),
        "{text}"
    );
}

#[test]
fn the_local_ca_signs_certificates_and_nothing_else() {
    let text = text_of(&LocalCa::generate().expect("deberia generarse"));

    assert!(text.contains("CA:TRUE, pathlen:0"), "{text}");
    assert!(
        text.contains("Certificate Sign, CRL Sign"),
        "el keyUsage se reduce a firmar certificados (ADR-0005):\n{text}"
    );
    assert!(
        !text.contains("Digital Signature"),
        "una CA local que sirva ademas para TLS es otra cosa:\n{text}"
    );
}

#[test]
fn the_local_ca_expires_between_two_and_three_years_from_now() {
    let ca = LocalCa::generate().expect("deberia generarse");

    let two_years = Asn1Time::days_from_now(2 * 365).expect("deberia calcularse");
    let three_years = Asn1Time::days_from_now(3 * 365).expect("deberia calcularse");
    assert!(
        ca.certificate().not_after() > two_years.as_ref(),
        "la caducidad debe superar los dos años"
    );
    assert!(
        ca.certificate().not_after() < three_years.as_ref(),
        "la caducidad debe ser menor a tres años"
    );
}

#[test]
fn two_local_ca_are_never_the_same_certificate() {
    let one = LocalCa::generate().expect("deberia generarse");
    let another = LocalCa::generate().expect("deberia generarse");

    assert_ne!(
        one.certificate().serial_number().to_bn().unwrap().to_vec(),
        another
            .certificate()
            .serial_number()
            .to_bn()
            .unwrap()
            .to_vec(),
        "el solape convive con dos CA locales vivas, y se distinguen por el serie"
    );
}

#[test]
fn a_local_ca_survives_the_round_trip_through_the_two_pem_files() {
    let original = LocalCa::generate().expect("deberia generarse");

    let restored = LocalCa::from_pem(
        &original.certificate_pem().expect("deberia salir en PEM"),
        &original.private_key_pem().expect("deberia salir en PEM"),
    )
    .expect("deberia releerse");

    assert_eq!(
        restored.certificate().to_pem().unwrap(),
        original.certificate().to_pem().unwrap()
    );
}

#[test]
fn a_certificate_with_someone_elses_key_is_damaged_material() {
    let one = LocalCa::generate().expect("deberia generarse");
    let another = LocalCa::generate().expect("deberia generarse");

    let error = LocalCa::from_pem(
        &one.certificate_pem().expect("deberia salir en PEM"),
        &another.private_key_pem().expect("deberia salir en PEM"),
    )
    .expect_err("un par que no se corresponde no es una CA local");

    assert_eq!(error.situation(), Situation::MaterialDamaged);
}

#[test]
fn the_stored_private_key_is_plain_pkcs8_and_not_the_certificate() {
    let ca = LocalCa::generate().expect("deberia generarse");

    let key = String::from_utf8(ca.private_key_pem().expect("deberia salir en PEM"))
        .expect("deberia ser UTF-8");

    assert!(key.starts_with("-----BEGIN PRIVATE KEY-----"), "{key}");
    assert!(!key.contains("ENCRYPTED"), "la clave se guarda sin cifrar");
}

#[test]
fn the_debug_output_never_carries_the_private_key() {
    let ca = LocalCa::generate().expect("deberia generarse");

    let printed = format!("{ca:?}");

    assert!(printed.contains("LocalCa"), "{printed}");
    assert!(!printed.contains("PRIVATE"), "{printed}");
}
