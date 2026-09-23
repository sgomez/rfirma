use super::super::*;
use super::fixtures::{a_sign_and_save, an_operation, dat, gzipped, properties, read_operation};
use crate::site::domain::protocol::XadesEnvelope;

#[test]
fn signing_and_saving_carries_its_document_and_the_round_that_cop_asks_for() {
    for (cop, round) in [
        (SIGN, SignatureRound::First),
        (COSIGN, SignatureRound::Again),
    ] {
        let SiteOperation::SignAndSave(request) =
            read_operation(&a_sign_and_save(cop, "")).expect("se atiende")
        else {
            panic!("es un firmar y guardar");
        };
        assert_eq!(request.round(), round, "con cop={cop}");
        assert_eq!(request.algorithm(), AskedAlgorithm::Sha256);
        assert_eq!(request.document(), Some(b"%PDF-1.7\n".as_slice()));
    }
}

#[test]
fn signing_and_saving_without_dat_leaves_the_document_to_be_chosen() {
    let url = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={SIGN}&format=PAdES&algorithm=SHA256withRSA"
    ));

    let SiteOperation::SignAndSave(request) = read_operation(&url).expect("dat es opcional") else {
        panic!("es un firmar y guardar");
    };
    assert_eq!(request.document(), None);
}

#[test]
fn signing_and_saving_without_dat_reads_the_selector_hints_from_properties() {
    let url = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={SIGN}&format=PAdES&algorithm=SHA256withRSA&properties={}",
        properties("filenameExts=pdf\nfilenameDescription=PDF\nfilenameCurrentDir=/home/persona\n")
    ));

    let SiteOperation::SignAndSave(request) = read_operation(&url).expect("dat es opcional") else {
        panic!("es un firmar y guardar");
    };
    assert_eq!(request.load_extensions(), ["pdf"]);
    assert_eq!(request.load_description(), Some("PDF"));
    assert_eq!(request.load_starting_folder(), Some("/home/persona"));
}

#[test]
fn a_chosen_document_under_format_auto_fixes_the_effective_format() {
    let url = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={SIGN}&format=auto&algorithm=SHA256withRSA"
    ));
    let SiteOperation::SignAndSave(request) = read_operation(&url).expect("dat es opcional") else {
        panic!("es un firmar y guardar");
    };

    let over_xml =
        request.with_chosen_document(b"<?xml version=\"1.0\"?><Facturae/>".to_vec(), None);
    assert_eq!(
        over_xml.format(),
        RequestedFormat::Xades(XadesEnvelope::Enveloping)
    );

    let over_pdf = request.with_chosen_document(b"%PDF-1.7\n".to_vec(), None);
    assert_eq!(over_pdf.document(), Some(b"%PDF-1.7\n".as_slice()));
    assert_eq!(over_pdf.format(), RequestedFormat::Pades);
}

#[test]
fn a_chosen_document_with_pades_explicit_keeps_the_format_the_site_named() {
    let url = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={SIGN}&format=PAdES&algorithm=SHA256withRSA"
    ));
    let SiteOperation::SignAndSave(request) = read_operation(&url).expect("dat es opcional") else {
        panic!("es un firmar y guardar");
    };

    let completed = request.with_chosen_document(b"lo que sea".to_vec(), None);
    assert_eq!(completed.document(), Some(b"lo que sea".as_slice()));
    assert_eq!(completed.format(), RequestedFormat::Pades);
}

#[test]
fn signing_and_saving_reads_its_three_filename_save_properties() {
    let url = a_sign_and_save(
        SIGN,
        &format!(
            "&filename=firma.pdf&properties={}",
            properties(
                "filenameSaveExts=pdf,p7s\nfilenameSaveDescription=Documentos\n\
                 filenameSaveCurrentDir=/home/persona\n"
            )
        ),
    );

    let SiteOperation::SignAndSave(request) = read_operation(&url).expect("se atiende") else {
        panic!("es un firmar y guardar");
    };
    assert_eq!(request.filename(), Some("firma.pdf"));
    assert_eq!(request.extensions(), ["pdf", "p7s"]);
    assert_eq!(request.description(), Some("Documentos"));
    assert_eq!(request.starting_folder(), Some("/home/persona"));
}

#[test]
fn signing_and_saving_rejects_an_algorithm_it_cannot_produce_like_sign_does() {
    let url = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={SIGN}&format=PAdES&algorithm=RIPEMD160withRSA&dat={}",
        dat(b"%PDF-1.7\n")
    ));

    let refusal = read_operation(&url).expect_err("rFirma no firma con RIPEMD160");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Algorithm));
}

#[test]
fn signing_and_saving_rejects_a_format_out_of_the_original_like_sign_does() {
    let url = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={SIGN}&format=OOXML&algorithm=SHA256withRSA&dat={}",
        dat(b"%PDF-1.7\n")
    ));

    let refusal = read_operation(&url).expect_err("OOXML no se atiende");

    assert_eq!(refusal.code(), SafCode::UnsupportedFormat);
}

#[test]
fn signing_and_saving_with_format_auto_reads_the_effective_format_over_a_pdf_or_an_xml() {
    let over_pdf = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={SIGN}&format=auto&algorithm=SHA256withRSA&dat={}",
        dat(b"%PDF-1.7\n")
    ));
    let SiteOperation::SignAndSave(request) = read_operation(&over_pdf).expect("es un PDF") else {
        panic!("es un firmar y guardar");
    };
    assert_eq!(request.document(), Some(b"%PDF-1.7\n".as_slice()));
    assert_eq!(request.format(), RequestedFormat::Pades);

    let over_xml = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={SIGN}&format=auto&algorithm=SHA256withRSA&dat={}",
        dat(b"<?xml version=\"1.0\"?><Facturae/>")
    ));
    let SiteOperation::SignAndSave(request) = read_operation(&over_xml).expect("es un XML") else {
        panic!("es un firmar y guardar");
    };
    assert_eq!(
        request.format(),
        RequestedFormat::Xades(XadesEnvelope::Enveloping)
    );
}

#[test]
fn signing_and_saving_with_an_unknown_cop_names_it() {
    let url = an_operation(&format!("op={SIGN_AND_SAVE}&cop=resign&format=PAdES"));

    let refusal = read_operation(&url).expect_err("'resign' no es 'sign' ni 'cosign'");

    assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
    assert!(refusal.detail().contains("resign"));
}

#[test]
fn signing_and_saving_with_countersign_is_refused_with_the_code_of_the_original() {
    let url = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={COUNTERSIGN}&format=PAdES"
    ));

    let refusal = read_operation(&url).expect_err("no existe en PAdES");

    assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
    assert!(refusal.detail().contains("countersign"));
}

#[test]
fn the_proposed_name_is_the_filename_of_the_site_when_it_came() {
    let SiteOperation::SignAndSave(request) =
        read_operation(&a_sign_and_save(SIGN, "&filename=contrato.pdf")).expect("se atiende")
    else {
        panic!("es un firmar y guardar");
    };
    assert_eq!(request.proposed_name(), "contrato.pdf");
}

#[test]
fn the_proposed_name_without_a_filename_is_the_default_of_the_original() {
    let SiteOperation::SignAndSave(request) =
        read_operation(&a_sign_and_save(SIGN, "")).expect("se atiende")
    else {
        panic!("es un firmar y guardar");
    };
    assert_eq!(request.proposed_name(), "Firma.pdf");
}

#[test]
fn the_proposed_name_without_a_filename_falls_back_to_the_chosen_document() {
    let SiteOperation::SignAndSave(request) =
        read_operation(&a_sign_and_save(SIGN, "")).expect("se atiende")
    else {
        panic!("es un firmar y guardar");
    };

    let with_chosen =
        request.with_chosen_document(b"lo que sea".to_vec(), Some("contrato.docx".to_owned()));
    assert_eq!(with_chosen.proposed_name(), "contrato.pdf");
}

#[test]
fn the_proposed_name_without_a_filename_infers_the_extension_from_the_format() {
    for (format, expected) in [
        ("PAdES", "Firma.pdf"),
        ("CAdES", "Firma.csig"),
        ("XAdES", "Firma.xsig"),
        ("CAdES-ASiC-S", "Firma.asics"),
        ("XAdES-ASiC-S", "Firma.asics"),
    ] {
        let url = an_operation(&format!(
            "op={SIGN_AND_SAVE}&cop={SIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format={format}&\
             algorithm=SHA256withRSA&dat={}",
            dat(b"documento")
        ));
        let SiteOperation::SignAndSave(request) = read_operation(&url).expect("se atiende") else {
            panic!("es un firmar y guardar");
        };
        assert_eq!(request.proposed_name(), expected, "con format={format}");
    }
}

#[test]
fn the_proposed_name_respects_the_filename_of_the_site_without_altering_it() {
    for (format, filename) in [
        ("PAdES", "contrato.pdf"),
        ("CAdES", "datos.bin"),
        ("XAdES", "factura.xml"),
        ("CAdES-ASiC-S", "archivo_sin_extension"),
    ] {
        let url = an_operation(&format!(
            "op={SIGN_AND_SAVE}&cop={SIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format={format}&\
             algorithm=SHA256withRSA&dat={}&filename={filename}",
            dat(b"documento")
        ));
        let SiteOperation::SignAndSave(request) = read_operation(&url).expect("se atiende") else {
            panic!("es un firmar y guardar");
        };
        assert_eq!(request.proposed_name(), filename, "con format={format}");
    }
}

#[test]
fn the_proposed_name_without_a_filename_falls_back_to_the_chosen_document_with_format_extension() {
    for (format, expected) in [
        ("PAdES", "contrato.pdf"),
        ("CAdES", "contrato.csig"),
        ("XAdES", "contrato.xsig"),
        ("CAdES-ASiC-S", "contrato.asics"),
    ] {
        let url = an_operation(&format!(
            "op={SIGN_AND_SAVE}&cop={SIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format={format}&\
             algorithm=SHA256withRSA"
        ));
        let SiteOperation::SignAndSave(request) = read_operation(&url).expect("se atiende") else {
            panic!("es un firmar y guardar");
        };
        let with_chosen =
            request.with_chosen_document(b"contenido".to_vec(), Some("contrato.docx".to_owned()));
        assert_eq!(with_chosen.proposed_name(), expected, "con format={format}");
    }
}

#[test]
fn the_proposed_name_of_the_site_wins_over_the_chosen_document() {
    let SiteOperation::SignAndSave(request) =
        read_operation(&a_sign_and_save(SIGN, "&filename=contrato.pdf")).expect("se atiende")
    else {
        panic!("es un firmar y guardar");
    };

    let with_chosen =
        request.with_chosen_document(b"lo que sea".to_vec(), Some("otro.docx".to_owned()));
    assert_eq!(with_chosen.proposed_name(), "contrato.pdf");
}

#[test]
fn signing_and_saving_proposes_the_actual_name_to_the_chooser() {
    let url = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={SIGN}&format=PAdES&algorithm=SHA256withRSA&properties={}",
        properties("filenameActualName=contrato.pdf\n")
    ));

    let SiteOperation::SignAndSave(request) = read_operation(&url).expect("se lee") else {
        panic!("es un signandsave");
    };

    assert_eq!(request.load_filename(), Some("contrato.pdf"));
}

#[test]
fn signing_and_saving_with_countersign_in_cades_is_attended() {
    let url = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={COUNTERSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format=CAdES&\
         algorithm=SHA256withRSA&dat={}",
        dat(b"una firma CAdES")
    ));

    let SiteOperation::SignAndSave(request) = read_operation(&url).expect("CAdES contrafirma")
    else {
        panic!("es un firmar y guardar");
    };

    assert_eq!(
        request.round(),
        SignatureRound::Counter {
            target: CounterTarget::Leafs
        }
    );
}

#[test]
fn signing_and_saving_with_countersign_in_xades_is_attended() {
    let url = an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={COUNTERSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format=XAdES&\
         algorithm=SHA256withRSA&dat={}",
        dat(b"<xml/>")
    ));

    let SiteOperation::SignAndSave(request) = read_operation(&url).expect("XAdES contrafirma")
    else {
        panic!("es un firmar y guardar");
    };

    assert_eq!(
        request.round(),
        SignatureRound::Counter {
            target: CounterTarget::Leafs
        }
    );
    assert_eq!(
        request.format(),
        RequestedFormat::Xades(XadesEnvelope::Enveloping)
    );
}

#[test]
fn gzip_true_decompresses_the_document_of_sign_and_save() {
    let plain = b"%PDF-1.7\nplain-pdf-content";
    let url = an_operation(&format!(
        "op=signandsave&cop=sign&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&algorithm=SHA256withRSA&gzip=true&dat={}",
        dat(&gzipped(plain))
    ));

    let SiteOperation::SignAndSave(request) = read_operation(&url).expect("se atiende") else {
        panic!("es signandsave");
    };
    assert_eq!(request.document(), Some(plain.as_slice()));
}
