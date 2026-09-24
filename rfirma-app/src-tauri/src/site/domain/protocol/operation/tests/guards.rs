use super::super::*;
use super::fixtures::{
    an_invoice_signature, an_operation, dat, properties, read_downloading, read_operation,
    ADownload, AN_INVOICE, A_REMOTE_DOCUMENT,
};
use crate::site::domain::protocol::{RefusalSituation, XadesEnvelope};
use crate::site::domain::triphase_server::ServerFormat;

#[test]
fn a_format_the_original_does_not_sign_in_three_phases_is_refused_by_the_protocol() {
    for name in ["OOXML", "ODF", "SOAP"] {
        let url = an_operation(&format!(
            "op=sign&format={name}&algorithm=SHA256withRSA&dat={}",
            dat(b"%PDF-1.7\n")
        ));

        let refusal = read_operation(&url).expect_err("no lo firma el original en tres fases");

        assert_eq!(refusal.code(), SafCode::UnsupportedFormat, "format={name}");
    }
}

/// **Grada A**: XMLDSig se queda fuera del alcance, y la razon esta en
/// `domain/protocol/mod.rs`.
#[test]
fn xmldsig_is_refused_by_the_protocol_because_the_original_has_no_triphase_for_it() {
    for name in [
        "XMLDSig",
        "XMLDSig Enveloping",
        "XMLDSig Detached",
        "XMLDSig Enveloped",
    ] {
        let url = an_operation(&format!(
            "op=sign&format={name}&algorithm=SHA256withRSA&dat={}",
            dat(b"<?xml version=\"1.0\"?><a/>")
        ));

        let refusal = read_operation(&url).expect_err("XMLDSig no se atiende");

        assert_eq!(refusal.code(), SafCode::UnsupportedFormat, "format={name}");
    }
}

#[test]
fn every_format_of_the_original_travels_as_the_closed_format_it_names() {
    for (name, expected) in [
        ("CAdES", RequestedFormat::Cades),
        (
            "XAdES Detached",
            RequestedFormat::Xades(XadesEnvelope::Detached),
        ),
        ("Factura-e", RequestedFormat::FacturaE),
        ("Adobe PDF", RequestedFormat::Pades),
    ] {
        let url = an_operation(&format!(
            "op=sign&format={name}&algorithm=SHA256withRSA&dat={}",
            dat(b"%PDF-1.7\n")
        ));

        let SiteOperation::Sign(request) = read_operation(&url).expect("se atiende") else {
            panic!("es una firma");
        };
        assert_eq!(request.format(), expected, "format={name}");
    }
}

#[test]
fn the_format_is_looked_at_before_anything_else_of_the_signature() {
    let url = an_operation("op=sign&format=OOXML&algorithm=loquesea");

    let refusal = read_operation(&url).expect_err("OOXML no se atiende");

    assert_eq!(refusal.code(), SafCode::UnsupportedFormat);
}

#[test]
fn format_auto_over_a_pdf_reads_as_pades_would_for_sign_and_cosign() {
    let signed_pdf = b"%PDF-1.7\n/ByteRange [0 10 20 30]\n";
    for verb in [SIGN, COSIGN] {
        let auto = an_operation(&format!(
            "op={verb}&idsession=8jAkPZfRw2mQxN4TbYuL&format=auto&algorithm=SHA256withRSA&dat={}",
            dat(signed_pdf)
        ));
        let explicit = an_operation(&format!(
            "op={verb}&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&algorithm=SHA256withRSA&dat={}",
            dat(signed_pdf)
        ));

        assert_eq!(
            read_operation(&auto).expect("un PDF con 'auto' se atiende"),
            read_operation(&explicit).expect("se atiende"),
            "'{verb}' con format=auto sobre un PDF"
        );
    }
}

#[test]
fn cosign_with_format_auto_over_unsigned_binary_is_refused_with_saf_17() {
    let url = an_operation(&format!(
        "op={COSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format=auto&algorithm=SHA256withRSA&dat={}",
        dat(&[0x00, 0x01, 0x02, 0x03])
    ));

    let refusal = read_operation(&url).expect_err("binario no firmado en cosign");

    assert_eq!(refusal.code(), SafCode::UnknownSigner);
}

#[test]
fn countersign_with_format_auto_over_unsigned_data_is_refused_with_saf_17() {
    for (document, label) in [
        (&[0x00, 0x01, 0x02, 0x03][..], "binario"),
        (b"%PDF-1.7\n".as_slice(), "PDF no firmado"),
        (
            b"<?xml version=\"1.0\"?><documento/>".as_slice(),
            "XML no firmado",
        ),
    ] {
        let url = an_operation(&format!(
            "op={COUNTERSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format=auto&algorithm=SHA256withRSA&dat={}",
            dat(document)
        ));

        let refusal = read_operation(&url).expect_err(label);

        assert_eq!(refusal.code(), SafCode::UnknownSigner, "{label}");
    }
}

#[test]
fn cosign_with_format_auto_over_unsigned_pdf_or_xml_is_refused_with_saf_17() {
    for (document, label) in [
        (b"%PDF-1.7\n".as_slice(), "PDF no firmado"),
        (
            b"<?xml version=\"1.0\"?><documento>sin firma</documento>".as_slice(),
            "XML no firmado",
        ),
    ] {
        let url = an_operation(&format!(
            "op={COSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format=auto&algorithm=SHA256withRSA&dat={}",
            dat(document)
        ));

        let refusal = read_operation(&url).expect_err(label);

        assert_eq!(refusal.code(), SafCode::UnknownSigner, "{label}");
    }
}

#[test]
fn multisig_with_format_auto_over_signed_documents_resolves_expected_formats() {
    let signed_pdf = b"%PDF-1.7\n/ByteRange [0 10 20 30]\n";
    let signed_xml =
        include_bytes!("../../../../../../../../testdata/reference/xades-enveloping.xml");
    let signed_cades =
        include_bytes!("../../../../../../../../testdata/reference/cades-implicit.p7s");

    let cosign_pdf = an_operation(&format!(
        "op={COSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format=auto&algorithm=SHA256withRSA&dat={}",
        dat(signed_pdf)
    ));
    let SiteOperation::Sign(request) = read_operation(&cosign_pdf).expect("PDF firmado en cosign")
    else {
        panic!("es una firma");
    };
    assert_eq!(request.format(), RequestedFormat::Pades);
    assert_eq!(request.round(), SignatureRound::Again);

    let cosign_xml = an_operation(&format!(
        "op={COSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format=auto&algorithm=SHA256withRSA&dat={}",
        dat(signed_xml)
    ));
    let SiteOperation::Sign(request) = read_operation(&cosign_xml).expect("XML firmado en cosign")
    else {
        panic!("es una firma");
    };
    assert_eq!(
        request.format(),
        RequestedFormat::Xades(XadesEnvelope::Enveloping)
    );
    assert_eq!(request.round(), SignatureRound::Again);

    let countersign_xml = an_operation(&format!(
        "op={COUNTERSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format=auto&algorithm=SHA256withRSA&dat={}",
        dat(signed_xml)
    ));
    let SiteOperation::Sign(request) =
        read_operation(&countersign_xml).expect("XML firmado en countersign")
    else {
        panic!("es una firma");
    };
    assert_eq!(
        request.format(),
        RequestedFormat::Xades(XadesEnvelope::Enveloping)
    );
    assert!(matches!(request.round(), SignatureRound::Counter { .. }));

    let cosign_cades = an_operation(&format!(
        "op={COSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format=auto&algorithm=SHA256withRSA&dat={}",
        dat(signed_cades)
    ));
    let SiteOperation::Sign(request) =
        read_operation(&cosign_cades).expect("CAdES firmado en cosign")
    else {
        panic!("es una firma");
    };
    assert_eq!(request.format(), RequestedFormat::Cades);
    assert_eq!(request.round(), SignatureRound::Again);

    let countersign_cades = an_operation(&format!(
        "op={COUNTERSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format=auto&algorithm=SHA256withRSA&dat={}",
        dat(signed_cades)
    ));
    let SiteOperation::Sign(request) =
        read_operation(&countersign_cades).expect("CAdES firmado en countersign")
    else {
        panic!("es una firma");
    };
    assert_eq!(request.format(), RequestedFormat::Cades);
    assert!(matches!(request.round(), SignatureRound::Counter { .. }));
}

#[test]
fn format_auto_over_xml_or_binary_reads_the_format_of_the_document() {
    for (document, expected) in [
        (
            b"<?xml version=\"1.0\"?><Facturae/>".as_slice(),
            RequestedFormat::Xades(XadesEnvelope::Enveloping),
        ),
        (&[0x00, 0x01, 0x02], RequestedFormat::Cades),
    ] {
        let url = an_operation(&format!(
            "op=sign&format=auto&algorithm=SHA256withRSA&dat={}",
            dat(document)
        ));

        let SiteOperation::Sign(request) = read_operation(&url).expect("se atiende") else {
            panic!("es una firma");
        };
        assert_eq!(request.format(), expected);
    }
}

/// Sin `dat` no hay nada sobre lo que detectar el formato, y el original no lo rechaza: espera
/// al documento que elija la persona (`ProtocolInvocationLauncherSign`, 1.9.2).
#[test]
fn format_auto_without_data_waits_for_the_document_instead_of_refusing() {
    let url = an_operation("op=sign&format=auto&algorithm=SHA256withRSA");

    let operation = read_operation(&url).expect("sin 'dat' se pide el documento");

    assert!(matches!(operation, SiteOperation::SignWithoutDocument(_)));
}

#[test]
fn every_algorithm_the_published_client_sends_is_typed_or_refused_with_the_code_of_the_original() {
    for (name, expected) in [
        ("SHA256", Some(AskedAlgorithm::Sha256)),
        ("SHA384", Some(AskedAlgorithm::Sha384)),
        ("SHA512", Some(AskedAlgorithm::Sha512)),
        ("SHA-256", Some(AskedAlgorithm::Sha256)),
        ("SHA-384", Some(AskedAlgorithm::Sha384)),
        ("SHA-512", Some(AskedAlgorithm::Sha512)),
        ("SHA256withRSA", Some(AskedAlgorithm::Sha256)),
        ("SHA-256withRSA", Some(AskedAlgorithm::Sha256)),
        ("SHA384withRSA", Some(AskedAlgorithm::Sha384)),
        ("SHA-384withRSA", Some(AskedAlgorithm::Sha384)),
        ("SHA512withRSA", Some(AskedAlgorithm::Sha512)),
        ("SHA-512withRSA", Some(AskedAlgorithm::Sha512)),
        ("SHA256withECDSA", Some(AskedAlgorithm::Sha256)),
        ("SHA-256withECDSA", Some(AskedAlgorithm::Sha256)),
        ("SHA384withECDSA", Some(AskedAlgorithm::Sha384)),
        ("SHA-384withECDSA", Some(AskedAlgorithm::Sha384)),
        ("SHA512withECDSA", Some(AskedAlgorithm::Sha512)),
        ("SHA-512withECDSA", Some(AskedAlgorithm::Sha512)),
        ("SHA256withDSA", Some(AskedAlgorithm::Sha256)),
        ("SHA1", Some(AskedAlgorithm::Sha1)),
        ("SHA-1", Some(AskedAlgorithm::Sha1)),
        ("SHA1withRSA", Some(AskedAlgorithm::Sha1)),
        ("SHA-1withRSA", Some(AskedAlgorithm::Sha1)),
        ("SHA1withECDSA", Some(AskedAlgorithm::Sha1)),
        ("MD5withRSA", None),
    ] {
        let url = an_operation(&format!(
            "op=sign&format=PAdES&algorithm={name}&dat={}",
            dat(b"%PDF-1.7\n")
        ));

        match (read_operation(&url), expected) {
            (Ok(SiteOperation::Sign(request)), Some(asked)) => {
                assert_eq!(request.algorithm(), asked, "{name}");
            }
            (Err(refusal), None) => {
                assert_eq!(refusal.code(), SafCode::Params, "{name}");
                assert_eq!(refusal.blame(), Some(Parameter::Algorithm), "{name}");
            }
            (other, _) => panic!("{name} no se atiende como toca: {other:?}"),
        }
    }
}

#[test]
fn an_algorithm_rfirma_cannot_produce_names_its_parameter() {
    let url = an_operation(&format!(
        "op=sign&format=PAdES&algorithm=RIPEMD160withRSA&dat={}",
        dat(b"%PDF-1.7\n")
    ));

    let refusal = read_operation(&url).expect_err("rFirma no firma con RIPEMD160");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Algorithm));
}

fn explicit() -> Vec<(String, String)> {
    vec![("mode".to_owned(), "explicit".to_owned())]
}

fn explicit_with_a_manifest() -> Vec<(String, String)> {
    vec![
        ("mode".to_owned(), "explicit".to_owned()),
        ("useManifest".to_owned(), "true".to_owned()),
    ]
}

const ENVELOPING: RequestedFormat = RequestedFormat::Xades(XadesEnvelope::Enveloping);

#[test]
fn explicit_mode_with_xades_is_refused_with_saf_06() {
    let refusal = refuse_explicit_xades(SignatureRound::First, ENVELOPING, None, &explicit())
        .expect_err("la XAdES explicita firma la huella SHA-1");

    assert_eq!(refusal.code(), SafCode::UnsupportedFormat);
    assert!(refusal.detail().contains("mode=explicit"));
}

#[test]
fn signing_and_saving_an_explicit_xades_is_refused_with_saf_06() {
    let url = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={SIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format=XAdES&\
         algorithm=SHA256withRSA&properties={}&dat={}",
        properties("mode=explicit"),
        dat(b"<a/>")
    ));
    let SiteOperation::SignAndSave(request) = read_operation(&url).expect("se lee") else {
        panic!("es un signandsave");
    };

    let refusal = refuse_explicit_xades(
        request.round(),
        request.format(),
        request.through_the_site_server(),
        request.declared_params(),
    )
    .expect_err("signandsave con XAdES explicita firma la huella SHA-1");

    assert_eq!(refusal.code(), SafCode::UnsupportedFormat);
}

#[test]
fn explicit_mode_with_pades_is_not_refused_here() {
    refuse_explicit_xades(
        SignatureRound::First,
        RequestedFormat::Pades,
        None,
        &explicit(),
    )
    .expect("PAdES no tiene esta desviacion");
}

#[test]
fn implicit_mode_with_xades_is_not_refused() {
    refuse_explicit_xades(
        SignatureRound::First,
        RequestedFormat::Xades(XadesEnvelope::Detached),
        None,
        &[("mode".to_owned(), "implicit".to_owned())],
    )
    .expect("solo se rechaza el modo explicito");
}

#[test]
fn an_explicit_xades_cosignature_is_not_refused() {
    refuse_explicit_xades(SignatureRound::Again, ENVELOPING, None, &explicit())
        .expect("AutoFirma solo firma la huella en sign");
}

#[test]
fn an_explicit_xades_countersignature_is_not_refused() {
    let round = SignatureRound::Counter {
        target: CounterTarget::Leafs,
    };

    refuse_explicit_xades(round, ENVELOPING, None, &explicit())
        .expect("AutoFirma solo firma la huella en sign");
}

#[test]
fn an_explicit_xadestri_signature_is_not_refused() {
    refuse_explicit_xades(
        SignatureRound::First,
        ENVELOPING,
        Some(ServerFormat::Xades),
        &explicit(),
    )
    .expect("AutoFirma no firma la huella en XAdEStri");
}

#[test]
fn an_explicit_xades_signature_with_a_manifest_is_not_refused() {
    refuse_explicit_xades(
        SignatureRound::First,
        ENVELOPING,
        None,
        &explicit_with_a_manifest(),
    )
    .expect("AutoFirma no firma la huella con useManifest=true");
}

#[test]
fn signing_an_invoice_is_attended_under_both_of_the_names_the_site_uses() {
    for format in ["FacturaE", "Factura-e"] {
        let SiteOperation::Sign(request) =
            read_operation(&an_invoice_signature(SIGN, format)).expect("se atiende")
        else {
            panic!("es una firma");
        };

        assert_eq!(request.format(), RequestedFormat::FacturaE);
        assert_eq!(request.round(), SignatureRound::First);
    }
}

#[test]
fn cosigning_or_countersigning_an_invoice_is_refused_with_the_code_of_the_original() {
    for verb in [COSIGN, COUNTERSIGN] {
        let refusal =
            read_operation(&an_invoice_signature(verb, "FacturaE")).expect_err("no se multifirma");

        assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
        assert_eq!(refusal.situation(), RefusalSituation::InvoiceMultisignature);
    }
}

#[test]
fn cosigning_an_invoice_under_format_auto_is_refused_too() {
    let refusal = read_operation(&an_invoice_signature(COSIGN, AUTO))
        .expect_err("una factura detectada tampoco se cofirma");

    assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
}

#[test]
fn signing_and_saving_a_cosignature_of_an_invoice_is_refused_as_well() {
    let url = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={COSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format=FacturaE&\
         algorithm=SHA256withRSA&dat={}",
        dat(AN_INVOICE)
    ));

    let refusal = read_operation(&url).expect_err("no se multifirma");

    assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
}

#[test]
fn the_guard_lets_a_first_signature_of_an_invoice_and_any_round_of_the_rest_through() {
    refuse_a_multisignature_of_an_invoice(SignatureRound::First, RequestedFormat::FacturaE)
        .expect("una factura se firma una vez");
    refuse_a_multisignature_of_an_invoice(
        SignatureRound::Again,
        RequestedFormat::Xades(XadesEnvelope::Enveloping),
    )
    .expect("la guarda es solo de facturas");
}

#[test]
fn the_format_auto_is_resolved_over_the_downloaded_document() {
    let url = an_operation(&format!(
        "op=sign&idsession=8jAkPZfRw2mQxN4TbYuL&format=auto&algorithm=SHA256withRSA&dat={A_REMOTE_DOCUMENT}"
    ));

    let operation = read_downloading(
        &url,
        &ADownload {
            from: A_REMOTE_DOCUMENT,
            content: b"%PDF-1.7\ndownloaded".to_vec(),
        },
    )
    .expect("se atiende");

    let SiteOperation::Sign(request) = operation else {
        panic!("es una firma");
    };
    assert_eq!(request.format(), RequestedFormat::Pades);
}

#[test]
fn the_explicit_xades_refusal_is_shown_with_its_own_situation_and_detail() {
    let refusal = refuse_explicit_xades(SignatureRound::First, ENVELOPING, None, &explicit())
        .expect_err("la XAdES explicita firma la huella SHA-1");

    assert_eq!(refusal.situation(), RefusalSituation::ExplicitXades);
    assert_eq!(
        refusal.to_string(),
        "SAF_06: mode=explicit con XAdES (firma de la huella SHA-1)"
    );
    assert!(refusal.is_shown_before_it_is_answered());
}

#[test]
fn the_refusal_of_a_multisigned_invoice_is_shown_with_its_own_situation_and_detail() {
    let refusal =
        refuse_a_multisignature_of_an_invoice(SignatureRound::Again, RequestedFormat::FacturaE)
            .expect_err("una factura no se cofirma");

    assert_eq!(refusal.situation(), RefusalSituation::InvoiceMultisignature);
    assert_eq!(
        refusal.to_string(),
        "SAF_04: FacturaE no admite cofirma ni contrafirma"
    );
    assert!(refusal.is_shown_before_it_is_answered());
}

#[test]
fn the_refusal_of_a_countersignature_outside_cades_and_xades_is_shown_with_its_own_situation() {
    let round = SignatureRound::Counter {
        target: CounterTarget::Leafs,
    };
    let refusal = refuse_a_countersignature_outside_cades_and_xades(round, RequestedFormat::Pades)
        .expect_err("PAdES no contrafirma");

    assert_eq!(
        refusal.situation(),
        RefusalSituation::UnsupportedCountersignature
    );
    assert_eq!(
        refusal.to_string(),
        "SAF_04: contrafirma fuera de CAdES, CMS y XAdES"
    );
    assert!(refusal.is_shown_before_it_is_answered());
}
