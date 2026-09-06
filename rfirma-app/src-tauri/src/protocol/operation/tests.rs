use super::*;

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
    let url = AfirmaUrl::parse("afirma://batch?op=batch").expect("es del protocolo");

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
    assert_eq!(request.algorithm(), "SHA256withRSA");
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

#[test]
fn saving_files_by_order_of_a_site_is_refused_on_purpose() {
    for verb in [SAVE, SIGN_AND_SAVE] {
        let refusal = read_operation(&a_signature(verb, "")).expect_err("esta fuera");

        assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
        assert!(
            refusal.detail().contains("no guarda ficheros"),
            "«{verb}» se rechaza por lo que es, no por desconocido: {}",
            refusal.detail()
        );
    }
}

#[test]
fn a_format_that_is_not_pades_is_refused_as_an_unsupported_format() {
    let url = an_operation(&format!(
        "op=sign&format=XAdES&algorithm=SHA256withRSA&dat={}",
        dat(b"%PDF-1.7\n")
    ));

    let refusal = read_operation(&url).expect_err("solo PAdES");

    assert_eq!(refusal.code(), SafCode::UnsupportedFormat);
}

#[test]
fn the_format_is_looked_at_before_anything_else_of_the_signature() {
    let url = an_operation("op=sign&format=CAdES&algorithm=loquesea");

    let refusal = read_operation(&url).expect_err("solo PAdES");

    assert_eq!(refusal.code(), SafCode::UnsupportedFormat);
}

#[test]
fn an_algorithm_rfirma_cannot_produce_names_its_parameter() {
    let url = an_operation(&format!(
        "op=sign&format=PAdES&algorithm=SHA512withRSA&dat={}",
        dat(b"%PDF-1.7\n")
    ));

    let refusal = read_operation(&url).expect_err("solo SHA256withRSA");

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
    let url = an_operation("op=sign&format=PAdES&algorithm=SHA256&dat=%%%");

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
        read_operation(&url).expect("el criterio esta en la lista blanca")
    else {
        panic!("es una seleccion de certificado");
    };

    assert_eq!(
        request.filter().declared(),
        [("filters".to_owned(), "subject.contains:PEREZ".to_owned())]
    );
}

#[test]
fn a_criterion_outside_the_whitelist_refuses_the_whole_call() {
    let url = an_operation(&format!(
        "op=selectcert&properties={}",
        properties("filters=inventado:loquesea\n")
    ));

    let refusal = read_operation(&url).expect_err("el criterio no esta en la lista blanca");

    assert_eq!(refusal.code(), SafCode::Params);
}

#[test]
fn the_slash_of_the_plain_base64_alphabet_is_accepted_too() {
    let plain =
        base64::engine::general_purpose::STANDARD.encode("filters=subject.contains:OÑ\n");
    assert!(plain.contains('/'), "la carga util trae una barra: {plain}");
    let url = an_operation(&format!("op=selectcert&properties={plain}"));

    read_operation(&url).expect("se lee igual");
}

#[test]
fn a_plus_of_the_plain_base64_alphabet_never_makes_it_this_far() {
    let plain =
        base64::engine::general_purpose::STANDARD.encode("filters=subject.contains:þ\n");
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
