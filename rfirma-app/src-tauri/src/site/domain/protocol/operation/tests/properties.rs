use super::super::*;
use super::fixtures::{
    a_countersignature, a_sign_and_save, a_signature, an_operation, properties, read_operation,
};

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
fn a_criterion_outside_the_whitelist_is_not_refused_and_declares_no_filter() {
    let url = an_operation(&format!(
        "op=selectcert&properties={}",
        properties(
            "filters=inventado:loquesea
"
        )
    ));

    let SiteOperation::SelectCertificate(request) =
        read_operation(&url).expect("un criterio desconocido no rechaza la operacion")
    else {
        panic!("es una seleccion de certificado");
    };

    assert!(request.filter().declares_nothing());
}

#[test]
fn the_slash_of_the_plain_base64_alphabet_is_accepted_too() {
    let plain = base64::engine::general_purpose::STANDARD.encode("filters=subject.contains:OÑ\n");
    assert!(plain.contains('/'), "la carga util trae una barra: {plain}");
    let url = an_operation(&format!("op=selectcert&properties={plain}"));

    read_operation(&url).expect("se lee igual");
}

/// El `+` del alfabeto normal llega aquí convertido en espacio, y el bloque ya no se lee: no
/// tumba la operación, la deja sin parámetros adicionales.
#[test]
fn a_plus_of_the_plain_base64_alphabet_never_makes_it_this_far() {
    let plain = base64::engine::general_purpose::STANDARD.encode("filters=subject.contains:þ\n");
    assert!(plain.contains('+'), "la carga util trae un mas: {plain}");
    let url = an_operation(&format!("op=selectcert&properties={plain}"));

    let SiteOperation::SelectCertificate(request) =
        read_operation(&url).expect("el mas ya es un espacio, y aun asi se firma")
    else {
        panic!("es una seleccion de certificado");
    };

    assert!(request.filter().declares_nothing());
}

#[test]
fn properties_that_are_not_base64_do_not_refuse_the_operation() {
    let url = an_operation("op=selectcert&properties=!!!!");

    let SiteOperation::SelectCertificate(request) =
        read_operation(&url).expect("un 'properties' ilegible se descarta, no rechaza")
    else {
        panic!("es una seleccion de certificado");
    };

    assert!(
        request.filter().declares_nothing(),
        "sin 'properties' legible no hay filtro"
    );
}

#[test]
fn properties_that_are_base64_but_not_utf8_do_not_refuse_the_operation_either() {
    let encoded = base64::engine::general_purpose::URL_SAFE.encode([0xff, 0xfe, 0xfd]);
    let url = an_operation(&format!("op=selectcert&properties={encoded}"));

    let SiteOperation::SelectCertificate(request) =
        read_operation(&url).expect("un 'properties' que no es texto se descarta")
    else {
        panic!("es una seleccion de certificado");
    };

    assert!(request.filter().declares_nothing());
}

#[test]
fn a_signature_with_unreadable_properties_keeps_no_filter_and_no_extra_params() {
    let url = a_signature(SIGN, "&properties=!!!!");

    let SiteOperation::Sign(request) =
        read_operation(&url).expect("un 'properties' ilegible se descarta, no rechaza")
    else {
        panic!("es una firma");
    };

    assert!(request.filter().declares_nothing());
    assert!(request.declared_params().is_empty());
}

#[test]
fn a_countersignature_with_unreadable_properties_falls_back_to_the_leafs_of_the_original() {
    let url = a_countersignature("CAdES", "&properties=!!!!");

    let SiteOperation::Sign(request) = read_operation(&url).expect("se contrafirma igual") else {
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
fn the_profile_of_the_site_never_reaches_the_signer() {
    let url = a_signature(
        SIGN,
        &format!(
            "&properties={}",
            properties(
                "profile=baseline
mode=implicit
"
            )
        ),
    );

    let SiteOperation::Sign(request) = read_operation(&url).expect("se firma") else {
        panic!("es una firma");
    };

    assert_eq!(
        request.declared_params(),
        [("mode".to_owned(), "implicit".to_owned())]
    );
}

#[test]
fn the_four_keys_the_launcher_reads_itself_never_reach_the_signer() {
    let url = a_signature(
        SIGN,
        &format!(
            "&properties={}",
            properties(
                "headless=true\nmandatoryCertSelection=false\nprofile=baseline\nfilenameActualName=contrato.pdf\n"
            )
        ),
    );

    let SiteOperation::Sign(request) = read_operation(&url).expect("se firma") else {
        panic!("es una firma");
    };

    assert!(
        request.declared_params().is_empty(),
        "ninguna de las cuatro cruza: {:?}",
        request.declared_params()
    );
}

/// La selección de certificado con el `properties` dado.
fn a_selection_declaring(declared: &str) -> SelectCertificate {
    let url = an_operation(&format!(
        "op=selectcert&properties={}",
        properties(declared)
    ));
    let SiteOperation::SelectCertificate(request) = read_operation(&url).expect("se lee") else {
        panic!("es una seleccion de certificado");
    };
    request
}

#[test]
fn headless_waives_the_choice_and_asks_nothing_else() {
    let request = a_selection_declaring("headless=true\n");

    assert!(request.waives_the_choice());
    assert!(request.is_headless());
}

#[test]
fn a_mandatory_certificate_selection_set_to_false_waives_the_choice_but_is_not_headless() {
    let request = a_selection_declaring("mandatoryCertSelection=FALSE\n");

    assert!(request.waives_the_choice());
    assert!(
        !request.is_headless(),
        "las demas preguntas se siguen haciendo"
    );
}

#[test]
fn a_mandatory_certificate_selection_set_to_true_keeps_the_dialog() {
    let request = a_selection_declaring("mandatoryCertSelection=true\n");

    assert!(!request.waives_the_choice());
    assert!(!request.is_headless());
}

#[test]
fn every_signature_reads_both_parameters_from_its_properties() {
    let waived = format!(
        "&properties={}",
        properties("mandatoryCertSelection=false\n")
    );
    let urls = [
        a_signature(SIGN, &waived),
        a_signature(COSIGN, &waived),
        a_countersignature("CAdES", &waived),
    ];

    for url in urls {
        let SiteOperation::Sign(request) = read_operation(&url).expect("se firma") else {
            panic!("es una firma: {url:?}");
        };

        assert!(request.waives_the_choice(), "{url:?}");
        assert!(!request.is_headless(), "{url:?}");
    }
}

#[test]
fn a_sign_and_save_reads_both_parameters_from_its_properties() {
    let url = a_sign_and_save(
        SIGN,
        &format!("&properties={}", properties("headless=true\n")),
    );

    let SiteOperation::SignAndSave(request) = read_operation(&url).expect("se lee") else {
        panic!("es un signandsave");
    };

    assert!(request.waives_the_choice());
    assert!(request.is_headless());
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

fn pair(key: &str, value: &str) -> (String, String) {
    (key.to_owned(), value.to_owned())
}

#[test]
fn a_line_ending_in_a_backslash_joins_the_next_one_without_its_leading_blanks() {
    assert_eq!(
        pairs_of("mode=impl\\\n    icit\r\nother=a\\\r\n\tb\n"),
        vec![pair("mode", "implicit"), pair("other", "ab")]
    );
}

#[test]
fn an_escaped_backslash_at_the_end_of_a_line_does_not_continue_it() {
    assert_eq!(
        pairs_of("a=b\\\\\nc=d\n"),
        vec![pair("a", "b\\"), pair("c", "d")]
    );
}

#[test]
fn a_key_can_be_split_by_a_continuation() {
    assert_eq!(pairs_of("ke\\\ny=v\n"), vec![pair("key", "v")]);
}

#[test]
fn a_unicode_escape_is_decoded_in_keys_and_values() {
    assert_eq!(
        pairs_of("\\u006dode=impl\\u0069cit\nemoji=\\uD83D\\uDE00\n"),
        vec![pair("mode", "implicit"), pair("emoji", "\u{1F600}")]
    );
}

#[test]
fn a_malformed_unicode_escape_discards_the_whole_block() {
    assert_eq!(pairs_of("a=b\nc=\\u12G4\n"), Vec::<(String, String)>::new());
}

#[test]
fn the_other_escapes_of_properties_load_are_decoded() {
    assert_eq!(
        pairs_of("a\\ b=c\\:d\\=e\\ f\\qg\\fh\n"),
        vec![pair("a b", "c:d=e fqg\u{c}h")]
    );
}

#[test]
fn a_blank_separates_key_and_value() {
    assert_eq!(
        pairs_of("a b\nc \t d\ne  =  f\ng :h\nlonely\n"),
        vec![
            pair("a", "b"),
            pair("c", "d"),
            pair("e", "f"),
            pair("g", "h"),
            pair("lonely", ""),
        ]
    );
}

#[test]
fn only_the_first_separator_after_the_blanks_counts() {
    assert_eq!(pairs_of("a = = b\n"), vec![pair("a", "= b")]);
}

#[test]
fn a_comment_line_is_not_continued() {
    assert_eq!(pairs_of("# nota \\\na=b\n"), vec![pair("a", "b")]);
}

#[test]
fn the_loading_keys_of_the_properties_govern_the_chooser_of_a_signature() {
    let url = an_operation(&format!(
        "op=sign&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&algorithm=SHA256withRSA&properties={}",
        properties(
            "filenameExts=pdf,p7s\nfilenameDescription=Documentos\nfilenameCurrentDir=/tmp/sede\n"
        )
    ));

    let SiteOperation::SignWithoutDocument(request) = read_operation(&url).expect("se atiende")
    else {
        panic!("sin 'dat' se pide el documento");
    };
    assert_eq!(
        request.load_extensions(),
        ["pdf".to_owned(), "p7s".to_owned()]
    );
    assert_eq!(request.load_description(), Some("Documentos"));
    assert_eq!(request.load_starting_folder(), Some("/tmp/sede"));
}
