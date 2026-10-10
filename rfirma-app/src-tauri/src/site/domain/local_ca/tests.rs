use super::*;

fn text_of(ca: &LocalCa) -> String {
    String::from_utf8(ca.certificate().to_text().expect("deberia imprimirse"))
        .expect("deberia ser UTF-8")
}

#[test]
fn the_local_ca_can_only_vouch_for_the_loopback() {
    let text = text_of(&LocalCa::generate(ChannelMark::Native).expect("deberia generarse"));

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
    let text = text_of(&LocalCa::generate(ChannelMark::Native).expect("deberia generarse"));

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
    let ca = LocalCa::generate(ChannelMark::Native).expect("deberia generarse");

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
    let one = LocalCa::generate(ChannelMark::Native).expect("deberia generarse");
    let another = LocalCa::generate(ChannelMark::Native).expect("deberia generarse");

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
    let original = LocalCa::generate(ChannelMark::Native).expect("deberia generarse");

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
    let one = LocalCa::generate(ChannelMark::Native).expect("deberia generarse");
    let another = LocalCa::generate(ChannelMark::Native).expect("deberia generarse");

    let error = LocalCa::from_pem(
        &one.certificate_pem().expect("deberia salir en PEM"),
        &another.private_key_pem().expect("deberia salir en PEM"),
    )
    .expect_err("un par que no se corresponde no es una CA local");

    assert_eq!(error.situation(), Situation::MaterialDamaged);
}

#[test]
fn the_stored_private_key_is_plain_pkcs8_and_not_the_certificate() {
    let ca = LocalCa::generate(ChannelMark::Native).expect("deberia generarse");

    let key = String::from_utf8(ca.private_key_pem().expect("deberia salir en PEM"))
        .expect("deberia ser UTF-8");

    assert!(key.starts_with("-----BEGIN PRIVATE KEY-----"), "{key}");
    assert!(!key.contains("ENCRYPTED"), "la clave se guarda sin cifrar");
}

#[test]
fn the_debug_output_never_carries_the_private_key() {
    let ca = LocalCa::generate(ChannelMark::Native).expect("deberia generarse");

    let printed = format!("{ca:?}");

    assert!(printed.contains("LocalCa"), "{printed}");
    assert!(!printed.contains("PRIVATE"), "{printed}");
}

fn der_of(ca: &LocalCa) -> Vec<u8> {
    ca.certificate().to_der().expect("deberia salir en DER")
}

#[test]
fn a_fresh_local_ca_carries_its_channel_in_a_non_critical_extension_of_its_own() {
    let text = text_of(&LocalCa::generate(ChannelMark::Flatpak).expect("deberia generarse"));

    assert!(
        text.contains(&format!("{CHANNEL_MARK_OID}: \n")),
        "la marca va en una extension no critica con el OID propio:\n{text}"
    );
    assert!(text.contains("flatpak"), "{text}");
}

#[test]
fn the_channel_mark_hangs_from_the_uuid_arc() {
    assert!(CHANNEL_MARK_OID.starts_with("2.25."), "{CHANNEL_MARK_OID}");
}

#[test]
fn the_channel_mark_leaves_the_subject_as_it_was() {
    let ca = LocalCa::generate(ChannelMark::Native).expect("deberia generarse");

    let subject = ca
        .certificate()
        .subject_name()
        .entries()
        .map(|entry| String::from_utf8_lossy(entry.data().as_slice()).into_owned())
        .collect::<Vec<_>>();
    assert_eq!(subject, vec![COMMON_NAME.to_owned()]);
}

#[test]
fn the_mark_is_read_back_from_the_certificate_of_each_channel() {
    for mark in [
        ChannelMark::Native,
        ChannelMark::Flatpak,
        ChannelMark::Windows,
    ] {
        let ca = LocalCa::generate(mark).expect("deberia generarse");

        assert_eq!(ChannelMark::of_certificate(&der_of(&ca)), Some(mark));
        assert_eq!(ca.mark(), Some(mark));
    }
}

#[test]
fn a_local_ca_from_before_the_mark_reads_as_unmarked() {
    let ca = LocalCa::unmarked_for_test().expect("deberia generarse");

    assert_eq!(ChannelMark::of_certificate(&der_of(&ca)), None);
    assert_eq!(ca.mark(), None);
}

#[test]
fn what_is_not_a_certificate_carries_no_mark() {
    assert_eq!(ChannelMark::of_certificate(b"no es un certificado"), None);
}

#[test]
fn only_the_channels_with_nss_stores_replace_an_unmarked_local_ca() {
    assert!(ChannelMark::Native.replaces_an_unmarked_local_ca());
    assert!(ChannelMark::Flatpak.replaces_an_unmarked_local_ca());
    assert!(!ChannelMark::Windows.replaces_an_unmarked_local_ca());
}

fn self_signed_der(entries: &[(Nid, &str)]) -> Vec<u8> {
    self_signed_der_with(entries, None)
}

fn self_signed_der_with(entries: &[(Nid, &str)], extension: Option<X509Extension>) -> Vec<u8> {
    let key = generate_key().expect("deberia generarse la clave");
    let mut name = X509Name::builder().expect("deberia construirse el nombre");
    for (nid, value) in entries {
        name.append_entry_by_nid(*nid, value)
            .expect("deberia añadirse la entrada");
    }
    let name = name.build();
    let mut builder = X509::builder().expect("deberia construirse el certificado");
    builder.set_subject_name(&name).expect("sujeto");
    builder.set_issuer_name(&name).expect("emisor");
    builder.set_pubkey(&key).expect("clave");
    builder
        .set_not_before(&Asn1Time::days_from_now(0).expect("inicio"))
        .expect("inicio");
    builder
        .set_not_after(&Asn1Time::days_from_now(1).expect("fin"))
        .expect("fin");
    if let Some(extension) = extension {
        builder.append_extension(extension).expect("extension");
    }
    builder
        .sign(&key, MessageDigest::sha256())
        .expect("deberia firmarse");
    builder.build().to_der().expect("deberia salir en DER")
}

#[test]
fn a_generated_local_ca_carries_the_local_ca_subject() {
    let ca = LocalCa::generate(ChannelMark::Native).expect("deberia generarse");

    assert!(has_the_local_ca_subject(
        &ca.certificate().to_der().expect("deberia salir en DER")
    ));
}

#[test]
fn a_namesake_without_the_constraints_still_carries_the_local_ca_subject() {
    assert!(has_the_local_ca_subject(&self_signed_der(&[(
        Nid::COMMONNAME,
        COMMON_NAME
    )])));
}

#[test]
fn a_subject_with_more_than_the_local_ca_name_is_not_the_local_ca_subject() {
    assert!(!has_the_local_ca_subject(&self_signed_der(&[
        (Nid::COMMONNAME, COMMON_NAME),
        (Nid::ORGANIZATIONNAME, "Ajena"),
    ])));
    assert!(!has_the_local_ca_subject(&self_signed_der(&[
        (Nid::ORGANIZATIONNAME, "Ajena"),
        (Nid::COMMONNAME, COMMON_NAME),
    ])));
}

#[test]
fn another_common_name_is_not_the_local_ca_subject() {
    assert!(!has_the_local_ca_subject(&self_signed_der(&[(
        Nid::COMMONNAME,
        "Otra CA"
    )])));
    assert!(!has_the_local_ca_subject(&self_signed_der(&[(
        Nid::COMMONNAME,
        "rFirma CA local 2"
    )])));
}

#[test]
fn bytes_that_are_not_a_certificate_are_not_the_local_ca_subject() {
    assert!(!has_the_local_ca_subject(b"no es DER"));
}

fn constraints_permitting(dns_names: &[&str]) -> X509Extension {
    let mut subtrees = Vec::new();
    for dns_name in dns_names {
        subtrees.extend_from_slice(&tagged(0x30, &tagged(0x82, dns_name.as_bytes())));
    }
    let der = tagged(0x30, &tagged(0xa0, &subtrees));
    X509Extension::new_from_der(
        &Asn1Object::from_str("2.5.29.30").expect("OID"),
        true,
        &Asn1OctetString::new_from_bytes(&der).expect("contenido"),
    )
    .expect("deberia construirse la restriccion")
}

#[test]
fn a_local_ca_of_any_channel_or_of_none_is_ours() {
    for ca in [
        LocalCa::generate(ChannelMark::Native),
        LocalCa::generate(ChannelMark::Flatpak),
        LocalCa::unmarked_for_test(),
    ] {
        assert!(is_an_rfirma_ca(&der_of(&ca.expect("deberia generarse"))));
    }
}

#[test]
fn a_namesake_without_the_name_constraints_is_not_ours() {
    assert!(!is_an_rfirma_ca(&self_signed_der(&[(
        Nid::COMMONNAME,
        COMMON_NAME
    )])));
}

#[test]
fn a_namesake_whose_constraints_are_not_exactly_the_loopback_is_not_ours() {
    assert!(!is_an_rfirma_ca(&self_signed_der_with(
        &[(Nid::COMMONNAME, COMMON_NAME)],
        Some(constraints_permitting(&["localhost", "example.com"])),
    )));
    assert!(!is_an_rfirma_ca(&self_signed_der_with(
        &[(Nid::COMMONNAME, COMMON_NAME)],
        Some(constraints_permitting(&["localhost"])),
    )));
}

#[test]
fn our_constraints_under_another_subject_are_not_ours() {
    assert!(!is_an_rfirma_ca(&self_signed_der_with(
        &[(Nid::COMMONNAME, "Otra CA")],
        Some(name_constraints().expect("deberia construirse")),
    )));
}

#[test]
fn bytes_that_are_not_a_certificate_are_not_ours() {
    assert!(!is_an_rfirma_ca(b"no es DER"));
}
