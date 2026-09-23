use super::super::*;
use super::fixtures::{
    an_operation, dat, read_downloading, read_operation, refusal_of, ADownload, A_REMOTE_DOCUMENT,
};

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
fn a_save_of_a_url_downloads_what_it_has_to_write() {
    let url = an_operation(&format!("op=save&dat={A_REMOTE_DOCUMENT}"));

    let operation = read_downloading(
        &url,
        &ADownload {
            from: A_REMOTE_DOCUMENT,
            content: b"%PDF-1.7\nguardame".to_vec(),
        },
    )
    .expect("se atiende");

    let SiteOperation::Save(request) = operation else {
        panic!("es un guardado");
    };
    assert_eq!(request.data(), b"%PDF-1.7\nguardame");
}

#[test]
fn a_filename_with_a_forbidden_character_is_refused_when_saving() {
    for forbidden in ["%5C", "%2F", "%3A", "*", "%3F", "%22", "<", ">", "%7C"] {
        let refusal = refusal_of(&format!(
            "afirma://save?dat=ZmlybWFkbw&filename=doc{forbidden}pdf"
        ));

        assert_eq!(refusal.code(), SafCode::Params, "con {forbidden}");
        assert_eq!(
            refusal.blame(),
            Some(Parameter::Filename),
            "con {forbidden}"
        );
    }
}
#[test]
fn a_filename_with_a_forbidden_character_is_refused_when_signing_and_saving() {
    let refusal = refusal_of("afirma://signandsave?dat=ZmlybWFkbw&format=pades&algorithm=SHA256withRSA&filename=a%2Fb.pdf");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Filename));
}
#[test]
fn a_plain_filename_goes_through() {
    assert!(read_operation(
        &AfirmaUrl::parse("afirma://save?dat=ZmlybWFkbw&filename=documento.pdf")
            .expect("es del protocolo")
    )
    .is_ok());
}
#[test]
fn extensions_with_a_forbidden_character_are_refused() {
    for forbidden in ["%3B", "+", "%2F", "%3A", "*"] {
        let refusal = refusal_of(&format!(
            "afirma://save?dat=ZmlybWFkbw&exts=pdf{forbidden}xml"
        ));

        assert_eq!(refusal.code(), SafCode::Params, "con {forbidden}");
        assert_eq!(
            refusal.blame(),
            Some(Parameter::Extensions),
            "con {forbidden}"
        );
    }
}
#[test]
fn a_comma_separated_list_of_extensions_goes_through() {
    assert!(read_operation(
        &AfirmaUrl::parse("afirma://save?dat=ZmlybWFkbw&exts=pdf,xml").expect("es del protocolo")
    )
    .is_ok());
}
