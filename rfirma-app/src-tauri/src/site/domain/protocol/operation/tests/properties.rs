use super::super::*;
use super::fixtures::{a_countersignature, a_signature, an_operation, properties, read_operation};

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

#[test]
fn headless_says_the_site_settles_for_the_only_certificate() {
    let url = an_operation(&format!(
        "op=selectcert&properties={}",
        properties("headless=true\n")
    ));

    let SiteOperation::SelectCertificate(request) = read_operation(&url).expect("se lee") else {
        panic!("es una seleccion de certificado");
    };

    assert!(request.is_headless());
}

#[test]
fn a_mandatory_certificate_selection_set_to_false_says_the_same_as_headless() {
    let url = an_operation(&format!(
        "op=selectcert&properties={}",
        properties("mandatoryCertSelection=false\n")
    ));

    let SiteOperation::SelectCertificate(request) = read_operation(&url).expect("se lee") else {
        panic!("es una seleccion de certificado");
    };

    assert!(request.is_headless());
}

#[test]
fn a_mandatory_certificate_selection_set_to_true_keeps_the_dialog() {
    let url = an_operation(&format!(
        "op=selectcert&properties={}",
        properties("mandatoryCertSelection=true\n")
    ));

    let SiteOperation::SelectCertificate(request) = read_operation(&url).expect("se lee") else {
        panic!("es una seleccion de certificado");
    };

    assert!(!request.is_headless());
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
