use super::*;
use crate::site::domain::protocol::XadesEnvelope;

/// **Grada A**: se lee una cadena y sale una petición. No hay socket, ni
/// token, ni puente.
fn an_operation(parameters: &str) -> AfirmaUrl {
    AfirmaUrl::parse(&format!("afirma://selectcert?{parameters}")).expect("es del protocolo")
}

fn properties(text: &str) -> String {
    base64::engine::general_purpose::URL_SAFE.encode(text.as_bytes())
}

#[test]
fn the_verb_of_the_published_client_is_the_selection_of_a_certificate() {
    let operation = read_operation(&an_operation(
        "op=selectcert&idsession=8jAkPZfRw2mQxN4TbYuL",
    ))
    .expect("es una operacion que se atiende");

    let SiteOperation::SelectCertificate(request) = operation else {
        panic!("el verbo del cliente publicado es la seleccion de certificado");
    };
    assert!(
        request.filter().declares_nothing(),
        "sin 'properties' no hay filtro declarado"
    );
}

/// El verbo va dos veces en la URL, y manda el parámetro.
#[test]
fn the_parameter_wins_over_the_domain_of_the_url() {
    let url = AfirmaUrl::parse("afirma://sign?op=selectcert").expect("es del protocolo");

    read_operation(&url).expect("el 'op' es el que manda");
}

/// Y sin parámetro, el dominio basta.
#[test]
fn without_the_parameter_the_domain_of_the_url_is_the_verb() {
    read_operation(&an_operation("idsession=8jAkPZfRw2mQxN4TbYuL")).expect("el dominio vale");
}

#[test]
fn an_operation_that_is_not_attended_is_refused_with_the_code_of_the_original() {
    let url = AfirmaUrl::parse("afirma://noexiste?op=noexiste").expect("es del protocolo");

    let refusal = read_operation(&url).expect_err("no se atiende");

    assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
}

fn dat(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE.encode(bytes)
}

fn a_signature(verb: &str, extra: &str) -> AfirmaUrl {
    an_operation(&format!(
        "op={verb}&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&algorithm=SHA256withRSA&dat={}{extra}",
        dat(b"%PDF-1.7\n")
    ))
}

#[test]
fn a_signature_carries_its_format_its_algorithm_and_the_document() {
    let operation = read_operation(&a_signature(SIGN, "")).expect("se atiende");

    let SiteOperation::Sign(request) = operation else {
        panic!("es una firma");
    };
    assert_eq!(request.round(), SignatureRound::First);
    assert_eq!(request.algorithm(), AskedAlgorithm::Sha256);
    assert_eq!(request.document(), b"%PDF-1.7\n");
}

#[test]
fn a_cosignature_is_the_same_request_with_another_round() {
    let operation = read_operation(&a_signature(COSIGN, "")).expect("se atiende");

    let SiteOperation::Sign(request) = operation else {
        panic!("es una firma");
    };
    assert_eq!(request.round(), SignatureRound::Again);
}

#[test]
fn a_countersignature_in_pades_is_refused_with_the_code_of_the_original() {
    let refusal = read_operation(&a_signature(COUNTERSIGN, "")).expect_err("no existe");

    assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
    assert!(refusal.detail().contains("countersign"));
}

fn a_sign_and_save(cop: &str, extra: &str) -> AfirmaUrl {
    an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={cop}&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&\
         algorithm=SHA256withRSA&dat={}{extra}",
        dat(b"%PDF-1.7\n")
    ))
}

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
        "op={SIGN_AND_SAVE}&cop={SIGN}&format=PAdES&algorithm=SHA1withRSA&dat={}",
        dat(b"%PDF-1.7\n")
    ));

    let refusal = read_operation(&url).expect_err("rFirma no firma con SHA1");

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
fn a_save_carries_its_document_and_its_optional_dialog_hints() {
    let url = an_operation(&format!(
        "op=save&dat={}&title=Guardar&filename=firma.pdf&exts=pdf,p7s&desc=Documentos",
        dat(b"%PDF-1.7\n")
    ));

    let SiteOperation::Save(request) = read_operation(&url).expect("se atiende") else {
        panic!("es un guardado");
    };
    assert_eq!(request.data(), b"%PDF-1.7\n");
    assert_eq!(request.title(), Some("Guardar"));
    assert_eq!(request.filename(), Some("firma.pdf"));
    assert_eq!(request.extensions(), ["pdf", "p7s"]);
    assert_eq!(request.description(), Some("Documentos"));
}

#[test]
fn a_save_without_dat_names_the_parameter_like_the_original() {
    let url = an_operation("op=save&title=Guardar");

    let refusal = read_operation(&url).expect_err("dat es obligatorio");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Data));
}

#[test]
fn a_load_reads_all_its_optional_parameters() {
    let url = an_operation(
        "op=load&title=Cargar&exts=pdf, p7s&desc=Documentos&filePath=/home/persona&multiload=true",
    );

    let SiteOperation::Load(request) = read_operation(&url).expect("se atiende") else {
        panic!("es una carga");
    };
    assert_eq!(request.title(), Some("Cargar"));
    assert_eq!(request.extensions(), ["pdf", "p7s"]);
    assert_eq!(request.description(), Some("Documentos"));
    assert_eq!(request.starting_folder(), Some("/home/persona"));
    assert!(request.multiple());
}

#[test]
fn a_load_without_multiload_is_a_single_file_load() {
    let url = an_operation("op=load");

    let SiteOperation::Load(request) = read_operation(&url).expect("se atiende") else {
        panic!("es una carga");
    };
    assert!(!request.multiple());
    assert!(request.extensions().is_empty());
}

#[test]
fn multiload_follows_java_boolean_parse_boolean() {
    for (value, expected) in [
        ("true", true),
        ("TRUE", true),
        ("false", false),
        ("otra-cosa", false),
    ] {
        let url = an_operation(&format!("op=load&multiload={value}"));

        let SiteOperation::Load(request) = read_operation(&url).expect("se atiende") else {
            panic!("es una carga");
        };
        assert_eq!(request.multiple(), expected, "multiload={value}");
    }
}

#[test]
fn a_format_the_original_does_not_sign_in_three_phases_is_refused_by_the_protocol() {
    for name in ["OOXML", "ODF", "SOAP", "NONE"] {
        let url = an_operation(&format!(
            "op=sign&format={name}&algorithm=SHA256withRSA&dat={}",
            dat(b"%PDF-1.7\n")
        ));

        let refusal = read_operation(&url).expect_err("no lo firma el original en tres fases");

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
    for verb in [SIGN, COSIGN] {
        let auto = an_operation(&format!(
            "op={verb}&idsession=8jAkPZfRw2mQxN4TbYuL&format=auto&algorithm=SHA256withRSA&dat={}",
            dat(b"%PDF-1.7\n")
        ));
        let explicit = a_signature(verb, "");

        assert_eq!(
            read_operation(&auto).expect("un PDF con 'auto' se atiende"),
            read_operation(&explicit).expect("se atiende"),
            "'{verb}' con format=auto sobre un PDF"
        );
    }
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

#[test]
fn format_auto_without_data_names_the_parameter_instead_of_the_format() {
    let url = an_operation("op=sign&format=auto&algorithm=SHA256withRSA");

    let refusal = read_operation(&url).expect_err("sin 'dat' no hay nada que detectar");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Data));
}

#[test]
fn every_algorithm_the_published_client_sends_is_typed_or_refused_with_the_code_of_the_original() {
    for (name, expected) in [
        ("SHA256", Some(AskedAlgorithm::Sha256)),
        ("SHA384", Some(AskedAlgorithm::Sha384)),
        ("SHA512", Some(AskedAlgorithm::Sha512)),
        ("SHA256withRSA", Some(AskedAlgorithm::Sha256)),
        ("SHA384withRSA", Some(AskedAlgorithm::Sha384)),
        ("SHA512withRSA", Some(AskedAlgorithm::Sha512)),
        ("SHA256withECDSA", Some(AskedAlgorithm::Sha256)),
        ("SHA384withECDSA", Some(AskedAlgorithm::Sha384)),
        ("SHA512withECDSA", Some(AskedAlgorithm::Sha512)),
        ("SHA1", None),
        ("SHA1withRSA", None),
        ("SHA1withECDSA", None),
        ("SHA256withDSA", None),
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
        "op=sign&format=PAdES&algorithm=SHA1withRSA&dat={}",
        dat(b"%PDF-1.7\n")
    ));

    let refusal = read_operation(&url).expect_err("rFirma no firma con SHA1");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Algorithm));
}

#[test]
fn each_missing_parameter_of_a_signature_names_itself() {
    for (parameters, blamed) in [
        ("op=sign", Parameter::Format),
        ("op=sign&format=PAdES", Parameter::Algorithm),
        ("op=sign&format=PAdES&algorithm=SHA256", Parameter::Data),
    ] {
        let refusal = read_operation(&an_operation(parameters)).expect_err("falta uno");

        assert_eq!(refusal.code(), SafCode::Params);
        assert_eq!(refusal.blame(), Some(blamed), "en «{parameters}»");
    }
}

#[test]
fn a_document_that_is_not_base64_names_the_parameter_that_came_wrong() {
    let url = an_operation("op=sign&format=PAdES&algorithm=SHA256&dat=::%");

    let refusal = read_operation(&url).expect_err("no es Base64");

    assert_eq!(refusal.blame(), Some(Parameter::Data));
}

#[test]
fn a_signature_that_asks_for_a_local_file_never_gets_read() {
    let url = an_operation("op=sign&dat=file:/etc/passwd");

    let refusal = read_operation(&url).expect_err("no se leen ficheros locales");

    assert_eq!(refusal.blame(), Some(Parameter::Data));
}

#[test]
fn the_extra_params_of_the_site_arrive_whole_and_unexpanded() {
    let url = a_signature(
        SIGN,
        &format!(
            "&properties={}",
            properties("expPolicy=FirmaAGE\nfilters=subject.contains:PEREZ\n")
        ),
    );

    let SiteOperation::Sign(request) = read_operation(&url).expect("se atiende") else {
        panic!("es una firma");
    };
    assert!(request
        .declared_params()
        .contains(&("expPolicy".to_owned(), "FirmaAGE".to_owned())));
    assert_eq!(
        request.filter().declared(),
        [("filters".to_owned(), "subject.contains:PEREZ".to_owned())],
        "y los filtros salen del mismo bloque, igual que en selectcert"
    );
}

#[test]
fn a_signature_with_nothing_to_sign_says_exactly_that() {
    let url = an_operation("op=sign&format=PAdES&algorithm=SHA256&dat=%3D");

    let refusal = read_operation(&url).expect_err("no hay nada que firmar");

    assert_eq!(refusal.code(), SafCode::SignWithoutData);
}

#[test]
fn the_two_common_guards_are_checked_in_the_selection_of_a_certificate_too() {
    let too_new = read_operation(&an_operation("op=selectcert&mcv=99.9.9"))
        .expect_err("la sede exige una version que no se implementa");
    assert_eq!(too_new.code(), SafCode::MinimumVersionNonSatisfied);

    let local = read_operation(&an_operation("op=selectcert&dat=file:///etc/shadow"))
        .expect_err("pide leer un fichero del equipo");
    assert_eq!(local.code(), SafCode::Params);
    assert_eq!(local.blame(), Some(Parameter::Data));
}

#[test]
fn the_filter_travels_inside_the_properties_and_comes_out_untouched() {
    let url = an_operation(&format!(
        "op=selectcert&properties={}",
        properties("filters=subject.contains:PEREZ\n")
    ));

    let SiteOperation::SelectCertificate(request) =
        read_operation(&url).expect("es una operacion que se atiende")
    else {
        panic!("es una seleccion de certificado");
    };

    assert_eq!(
        request.filter().declared(),
        [("filters".to_owned(), "subject.contains:PEREZ".to_owned())]
    );
}

#[test]
fn a_criterion_outside_the_whitelist_reaches_the_engine_instead_of_refusing() {
    let url = an_operation(&format!(
        "op=selectcert&properties={}",
        properties(
            "filters=inventado:loquesea
"
        )
    ));

    let SiteOperation::SelectCertificate(request) =
        read_operation(&url).expect("un criterio desconocido lo juzga el motor")
    else {
        panic!("es una seleccion de certificado");
    };

    assert_eq!(
        request.filter().declared(),
        [("filters".to_owned(), "inventado:loquesea".to_owned())]
    );
}

#[test]
fn the_slash_of_the_plain_base64_alphabet_is_accepted_too() {
    let plain = base64::engine::general_purpose::STANDARD.encode("filters=subject.contains:OÑ\n");
    assert!(plain.contains('/'), "la carga util trae una barra: {plain}");
    let url = an_operation(&format!("op=selectcert&properties={plain}"));

    read_operation(&url).expect("se lee igual");
}

#[test]
fn a_plus_of_the_plain_base64_alphabet_never_makes_it_this_far() {
    let plain = base64::engine::general_purpose::STANDARD.encode("filters=subject.contains:þ\n");
    assert!(plain.contains('+'), "la carga util trae un mas: {plain}");
    let url = an_operation(&format!("op=selectcert&properties={plain}"));

    let refusal = read_operation(&url).expect_err("el mas ya es un espacio");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Properties));
}

#[test]
fn properties_that_are_not_base64_name_the_parameter_that_came_wrong() {
    let url = an_operation("op=selectcert&properties=!!!!");

    let refusal = read_operation(&url).expect_err("no es Base64");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Properties));
}

#[test]
fn the_properties_block_is_read_the_way_the_original_writes_it() {
    let pairs = pairs_of("# un comentario\n\nfilters=subject.rfc2254:\\(cn=X\\)\nkey:valor\n");

    assert_eq!(
        pairs,
        vec![
            ("filters".to_owned(), "subject.rfc2254:(cn=X)".to_owned()),
            ("key".to_owned(), "valor".to_owned()),
        ]
    );
}

#[test]
fn an_escaped_separator_does_not_split_the_line() {
    let pairs = pairs_of("cla\\=ve=valor\n");

    assert_eq!(pairs, vec![("cla=ve".to_owned(), "valor".to_owned())]);
}

#[test]
fn the_two_sticky_flags_travel_inside_the_selection_of_a_certificate() {
    let SiteOperation::SelectCertificate(plain) =
        read_operation(&an_operation("op=selectcert")).expect("es una operacion que se atiende")
    else {
        panic!("es una seleccion de certificado");
    };
    assert_eq!(plain.sticky(), StickyCertificate::default());

    let SiteOperation::SelectCertificate(stuck) =
        read_operation(&an_operation("op=selectcert&sticky=true&resetsticky=true"))
            .expect("es una operacion que se atiende")
    else {
        panic!("es una seleccion de certificado");
    };
    assert!(stuck.sticky().is_sticky());
    assert!(stuck.sticky().resets());
}

fn a_batch(extra: &str) -> AfirmaUrl {
    an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&\
         batchpresignerurl=https%3A%2F%2Fpresigner.example%2Fpre&\
         batchpostsignerurl=https%3A%2F%2Fpostsigner.example%2Fpost{extra}"
    ))
}

fn xml_lote(algorithm: &str, stop_on_error: bool) -> String {
    format!(
        "<signbatch algorithm=\"{algorithm}\" stoponerror=\"{stop_on_error}\">\
         <singlesign id=\"001\"/></signbatch>"
    )
}

fn json_lote(algorithm: &str, stop_on_error: bool) -> String {
    format!("{{\"algorithm\":\"{algorithm}\",\"stoponerror\":{stop_on_error}}}")
}

#[test]
fn a_batch_reads_its_two_servlets_and_the_algorithm_of_its_xml_lote() {
    let lote = xml_lote("SHA256", true);
    let url = a_batch(&format!("&dat={}", dat(lote.as_bytes())));

    let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
        panic!("es un lote");
    };
    assert_eq!(
        request.presigner_url(),
        Some("https://presigner.example/pre")
    );
    assert_eq!(
        request.postsigner_url(),
        Some("https://postsigner.example/post")
    );
    assert!(!request.is_local());
    assert!(!request.is_json());
    assert_eq!(request.algorithm(), "SHA256");
    assert!(request.stops_on_error());
}

#[test]
fn a_batch_with_jsonbatch_reads_the_algorithm_of_its_json_lote() {
    let lote = json_lote("sha1", false);
    let url = a_batch(&format!("&jsonbatch=true&dat={}", dat(lote.as_bytes())));

    let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
        panic!("es un lote");
    };
    assert!(request.is_json());
    assert_eq!(request.algorithm(), "sha1");
    assert!(!request.stops_on_error());
}

#[test]
fn a_batch_without_the_presigner_url_names_it() {
    let url = an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&\
         batchpostsignerurl=https%3A%2F%2Fpostsigner.example%2Fpost&dat={}",
        dat(xml_lote("SHA256", false).as_bytes())
    ));

    let refusal = read_operation(&url).expect_err("falta la url de prefirma");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::BatchPresignerUrl));
}

#[test]
fn a_batch_servlet_url_that_is_not_https_is_refused() {
    let url = an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&\
         batchpresignerurl=http%3A%2F%2Fpresigner.example%2Fpre&\
         batchpostsignerurl=https%3A%2F%2Fpostsigner.example%2Fpost&dat={}",
        dat(xml_lote("SHA256", false).as_bytes())
    ));

    let refusal = read_operation(&url).expect_err("http no es https");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::BatchPresignerUrl));
}

#[test]
fn a_local_batch_needs_no_servlet_urls() {
    let url = an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&localBatchProcess=true&jsonbatch=true&dat={}",
        dat(json_lote("SHA256", true).as_bytes())
    ));

    let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
        panic!("es un lote");
    };
    assert!(request.is_local());
    assert!(request.is_json());
    assert_eq!(request.presigner_url(), None);
    assert_eq!(request.postsigner_url(), None);
    assert!(request.stops_on_error());
}

/// El original manda el XML heredado a los servlets aunque la sede pida el lote
/// local; rFirma no lo atiende (ver el encabezado del módulo `protocol`).
#[test]
fn a_local_batch_in_the_legacy_xml_is_refused() {
    let url = an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&localBatchProcess=true&dat={}",
        dat(xml_lote("SHA256", false).as_bytes())
    ));

    let refusal = read_operation(&url).expect_err("el lote local solo existe en JSON");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Data));
}

#[test]
fn a_batch_algorithm_rfirma_cannot_produce_names_the_algorithm_parameter() {
    let url = a_batch(&format!("&dat={}", dat(xml_lote("MD5", false).as_bytes())));

    let refusal = read_operation(&url).expect_err("MD5 no esta en el catalogo del lote");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Algorithm));
}

#[test]
fn a_batch_reads_needcert_the_filter_and_the_sticky_flags() {
    let url = a_batch(&format!(
        "&needcert=true&sticky=true&resetsticky=true&dat={}",
        dat(xml_lote("SHA256", false).as_bytes())
    ));

    let SiteOperation::Batch(request) = read_operation(&url).expect("se atiende") else {
        panic!("es un lote");
    };
    assert!(request.needcert());
    assert!(request.filter().declares_nothing());
    assert!(request.sticky().is_sticky());
    assert!(request.sticky().resets());
    assert_eq!(
        request.lote_base64(),
        dat(xml_lote("SHA256", false).as_bytes())
    );
    assert_eq!(request.lote(), xml_lote("SHA256", false).as_bytes());
}

/// La contrafirma que pide una sede, con el formato y los `extraParams` que se le digan.
fn a_countersignature(format: &str, extra: &str) -> AfirmaUrl {
    an_operation(&format!(
        "op={COUNTERSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format={format}&\
         algorithm=SHA256withRSA&dat={}{extra}",
        dat(b"una firma")
    ))
}

#[test]
fn a_countersignature_in_cades_carries_the_target_the_site_declared() {
    let url = a_countersignature(
        "CAdES",
        &format!("&properties={}", properties("target=tree\n")),
    );

    let SiteOperation::Sign(request) = read_operation(&url).expect("CAdES contrafirma") else {
        panic!("es una firma");
    };

    assert_eq!(
        request.round(),
        SignatureRound::Counter {
            target: CounterTarget::Tree
        }
    );
    assert_eq!(request.format(), RequestedFormat::Cades);
}

#[test]
fn a_countersignature_without_a_target_counters_the_leafs_like_the_original() {
    let url = a_countersignature("CAdES", "");

    let SiteOperation::Sign(request) = read_operation(&url).expect("CAdES contrafirma") else {
        panic!("es una firma");
    };

    assert_eq!(
        request.round(),
        SignatureRound::Counter {
            target: CounterTarget::Leafs
        }
    );
}

#[test]
fn a_countersignature_under_format_auto_over_a_binary_is_a_cades_one() {
    let url = a_countersignature(AUTO, "");

    let SiteOperation::Sign(request) = read_operation(&url).expect("un binario es CAdES") else {
        panic!("es una firma");
    };

    assert_eq!(request.format(), RequestedFormat::Cades);
}

#[test]
fn a_countersignature_with_a_target_that_is_neither_tree_nor_leafs_is_refused() {
    let url = a_countersignature(
        "CAdES",
        &format!("&properties={}", properties("target=roots\n")),
    );

    let refusal = read_operation(&url).expect_err("solo hay dos objetivos");

    assert_eq!(refusal.code(), SafCode::Params);
    assert!(refusal.detail().contains("roots"));
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
fn a_countersignature_in_xades_carries_the_target_the_site_declared() {
    let url = a_countersignature(
        "XAdES",
        &format!("&properties={}", properties("target=tree\n")),
    );

    let SiteOperation::Sign(request) = read_operation(&url).expect("XAdES contrafirma") else {
        panic!("es una firma");
    };

    assert_eq!(
        request.round(),
        SignatureRound::Counter {
            target: CounterTarget::Tree
        }
    );
    assert_eq!(
        request.format(),
        RequestedFormat::Xades(XadesEnvelope::Enveloping)
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
fn a_countersignature_under_format_auto_over_an_xml_is_a_xades_one() {
    let url = an_operation(&format!(
        "op={COUNTERSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format={AUTO}&\
         algorithm=SHA256withRSA&dat={}",
        dat(b"<xml/>")
    ));

    let SiteOperation::Sign(request) = read_operation(&url).expect("un XML es XAdES") else {
        panic!("es una firma");
    };

    assert_eq!(
        request.format(),
        RequestedFormat::Xades(XadesEnvelope::Enveloping)
    );
}

#[test]
fn explicit_mode_with_xades_is_refused_with_saf_06() {
    let refusal = refuse_explicit_xades(
        RequestedFormat::Xades(XadesEnvelope::Enveloping),
        &[("mode".to_owned(), "explicit".to_owned())],
    )
    .expect_err("la XAdES explicita no se reproduce");

    assert_eq!(refusal.code(), SafCode::UnsupportedFormat);
    assert!(refusal.detail().contains("mode=explicit"));
}

#[test]
fn explicit_mode_with_pades_is_not_refused_here() {
    refuse_explicit_xades(
        RequestedFormat::Pades,
        &[("mode".to_owned(), "explicit".to_owned())],
    )
    .expect("PAdES no tiene esta desviacion");
}

#[test]
fn implicit_mode_with_xades_is_not_refused() {
    refuse_explicit_xades(
        RequestedFormat::Xades(XadesEnvelope::Detached),
        &[("mode".to_owned(), "implicit".to_owned())],
    )
    .expect("solo se rechaza el modo explicito");
}
