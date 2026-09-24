use super::super::*;
use super::fixtures::{
    a_signature, a_signature_order, an_invoice_signature, an_operation, an_order,
    code_of_the_order, dat, read_operation, refusal_of,
};

#[test]
fn an_operation_that_is_not_attended_is_refused_with_the_code_of_the_original() {
    let url = AfirmaUrl::parse("afirma://noexiste?op=noexiste").expect("es del protocolo");

    let refusal = read_operation(&url).expect_err("no se atiende");

    assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
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

#[test]
fn an_operation_leaves_the_protocol_version_it_declares_to_the_channel() {
    for verb in ["selectcert", "sign", "signandsave", "save", "load", "batch"] {
        let outcome = read_operation(&an_operation(&format!("op={verb}&ver=5")));

        assert!(
            outcome.as_ref().map_or_else(
                |refusal| refusal.code() != SafCode::UnsupportedProcedure,
                |_| true
            ),
            "con op={verb}: {outcome:?}"
        );
    }
}

/// El almacén se lee al final de la operación, como en el original (ADR-0022).
#[test]
fn a_signature_that_names_a_store_rfirma_does_not_open_is_refused() {
    let named = base64::engine::general_purpose::URL_SAFE.encode(b"PKCS12:/ruta/almacen.p12");

    let refusal = read_operation(&a_signature(SIGN, &format!("&ksb64={named}")))
        .expect_err("rFirma no abre ese almacen");

    assert_eq!(refusal.code(), SafCode::CannotAccessKeystore);
    assert_eq!(refusal.blame(), Some(Parameter::KeyStore));
}

#[test]
fn a_selection_that_names_the_store_rfirma_opens_goes_on() {
    let named = base64::engine::general_purpose::URL_SAFE.encode(b"SHARED_NSS");

    read_operation(&an_operation(&format!(
        "op=selectcert&idsession=8jAkPZfRw2mQxN4TbYuL&ksb64={named}"
    )))
    .expect("es el almacen que rFirma abre");
}

#[test]
fn a_selection_that_names_a_pkcs11_module_is_narrowed_to_it() {
    let named =
        base64::engine::general_purpose::URL_SAFE.encode(b"PKCS11:/usr/lib/softhsm/libsofthsm2.so");

    let asked = read_operation(&an_operation(&format!(
        "op=selectcert&idsession=8jAkPZfRw2mQxN4TbYuL&ksb64={named}"
    )))
    .expect("la acotacion la decide el adaptador");

    let SiteOperation::SelectCertificate(selection) = asked else {
        panic!("es una seleccion");
    };
    assert_eq!(
        selection.filter().module(),
        Some("/usr/lib/softhsm/libsofthsm2.so")
    );
}

#[test]
fn a_signature_that_names_a_pkcs11_module_is_narrowed_to_it() {
    let named =
        base64::engine::general_purpose::URL_SAFE.encode(b"PKCS11:/usr/lib/opensc-pkcs11.so");

    let asked = read_operation(&a_signature(SIGN, &format!("&ksb64={named}")))
        .expect("la acotacion la decide el adaptador");

    let SiteOperation::Sign(signature) = asked else {
        panic!("es una firma");
    };
    assert_eq!(
        signature.filter().module(),
        Some("/usr/lib/opensc-pkcs11.so")
    );
}

#[test]
fn a_selection_that_names_no_module_is_not_narrowed() {
    let asked = read_operation(&an_operation(
        "op=selectcert&idsession=8jAkPZfRw2mQxN4TbYuL",
    ))
    .expect("es una seleccion corriente");

    let SiteOperation::SelectCertificate(selection) = asked else {
        panic!("es una seleccion");
    };
    assert_eq!(selection.filter().module(), None);
}

/// Guardar y cargar no eligen certificado, y allí el original ni mira el almacén.
#[test]
fn a_save_that_names_a_store_is_not_refused_for_naming_it() {
    let named = base64::engine::general_purpose::URL_SAFE.encode(b"PKCS12:/ruta/almacen.p12");
    let url = an_operation(&format!("op=save&dat={}&ksb64={named}", dat(b"%PDF-1.7\n")));

    read_operation(&url).expect("un guardado no elige certificado");
}

/// El original mira antes el resto de parámetros: un `format` ausente gana al almacén.
#[test]
fn the_store_is_read_after_the_rest_of_the_parameters() {
    let named = base64::engine::general_purpose::URL_SAFE.encode(b"PKCS12");
    let url = an_operation(&format!(
        "op=sign&idsession=8jAkPZfRw2mQxN4TbYuL&ksb64={named}"
    ));

    let refusal = read_operation(&url).expect_err("falta el formato");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Format));
}

#[test]
fn a_selection_with_a_fileid_and_no_rtservlet_is_refused_as_a_parameter_error() {
    let url = an_order("afirma://selectcert/?fileid=abc123");

    assert_eq!(code_of_the_order(&url), SafCode::Params);
}

#[test]
fn a_batch_with_a_fileid_and_no_rtservlet_is_refused_as_a_parameter_error() {
    let url = an_order("afirma://batch/?fileid=abc123");

    assert_eq!(code_of_the_order(&url), SafCode::Params);
}

#[test]
fn a_signature_whose_id_is_not_alphanumeric_is_refused_as_a_parameter_error() {
    let url = an_order("afirma://sign?op=sign&id=rfirma-1&format=CAdES&algorithm=SHA256");

    let refusal = read_operation(&url).expect_err("el id no es alfanumerico");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Identifier));
}

#[test]
fn a_cipher_key_of_seven_characters_is_refused_as_a_parameter_error() {
    let url = an_order(&format!(
        "afirma://sign?op=sign&format=INVENTADO&algorithm=SHA256&dat={}&key=1234567",
        dat(b"rfirma")
    ));

    let refusal = read_operation(&url).expect_err("la clave no tiene ocho caracteres");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::CipherKey));
}

#[test]
fn a_signature_with_a_fileid_and_no_rtservlet_is_refused_before_its_format() {
    let url = a_signature_order("fileid=abc123");

    assert_eq!(code_of_the_order(&url), SafCode::Params);
}

#[test]
fn a_signature_with_a_local_file_in_dat_is_refused_before_its_format() {
    let url = a_signature_order("dat=file:/etc/hostname");

    assert_eq!(code_of_the_order(&url), SafCode::Params);
}

#[test]
fn an_rtservlet_over_ftp_is_refused_as_a_parameter_error() {
    let url = a_signature_order("fileid=abc123&rtservlet=ftp://sede.example/rt");

    let refusal = read_operation(&url).expect_err("ftp no es http ni https");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::RetrieveServlet));
}

#[test]
fn an_rtservlet_on_localhost_is_refused_as_a_local_access() {
    let url = a_signature_order("fileid=abc123&rtservlet=http://localhost/rt");

    assert_eq!(code_of_the_order(&url), SafCode::LocalAccessBlocked);
}

#[test]
fn an_rtservlet_on_the_loopback_address_is_refused_as_a_local_access() {
    let url = an_order("afirma://sign?op=sign&fileid=rfirma&rtservlet=http://127.0.0.1/rt");

    assert_eq!(code_of_the_order(&url), SafCode::LocalAccessBlocked);
}

#[test]
fn an_rtservlet_with_a_query_of_its_own_is_refused_as_a_parameter_error() {
    let url =
        a_signature_order("fileid=abc123&rtservlet=https%3A%2F%2Fsede.example%2Frt%3Fop%3Dget");

    assert_eq!(code_of_the_order(&url), SafCode::Params);
}

#[test]
fn a_malformed_minimum_client_version_is_refused_as_a_parameter_error() {
    let url = a_signature_order("dat=SG9sYQ&mcv=uno.dos");

    assert_eq!(code_of_the_order(&url), SafCode::Params);
}

#[test]
fn an_id_of_twenty_one_characters_is_refused_as_a_parameter_error() {
    let url = a_signature_order(&format!("dat=SG9sYQ&id={}", "a".repeat(21)));

    assert_eq!(code_of_the_order(&url), SafCode::Params);
}

#[test]
fn an_id_of_twenty_characters_reaches_the_format() {
    let url = a_signature_order(&format!("dat=SG9sYQ&id={}", "a".repeat(20)));

    assert_eq!(code_of_the_order(&url), SafCode::UnsupportedFormat);
}

#[test]
fn a_fileid_of_twenty_one_characters_is_refused_even_with_dat() {
    let url = a_signature_order(&format!("dat=SG9sYQ&fileid={}", "a".repeat(21)));

    let refusal = read_operation(&url).expect_err("sin id, el fileid es el identificador");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::FileId));
}

#[test]
fn the_id_wins_over_the_fileid_as_the_identifier_of_the_operation() {
    let url = a_signature_order(&format!("dat=SG9sYQ&id=abc123&fileid={}", "a".repeat(21)));

    assert_eq!(code_of_the_order(&url), SafCode::UnsupportedFormat);
}

#[test]
fn a_malformed_properties_or_ksb64_reaches_the_format() {
    for probed in ["properties=esto-no-es-base64!", "ksb64=esto-no-es-base64!"] {
        let url = a_signature_order(&format!("dat=SG9sYQ&{probed}"));

        assert_eq!(
            code_of_the_order(&url),
            SafCode::UnsupportedFormat,
            "{probed}"
        );
    }
}

#[test]
fn an_unknown_verb_of_the_channel_is_refused_as_unsupported() {
    let url = an_order("afirma://unknownop?op=unknownop");

    assert_eq!(code_of_the_order(&url), SafCode::UnsupportedOperation);
}

#[test]
fn an_invalid_op_over_a_signature_is_refused_as_unsupported_and_not_by_its_format() {
    let url = an_order(&format!(
        "afirma://sign?op=invalid&format=INVENTADO&algorithm=SHA256&dat={}",
        dat(b"rfirma")
    ));

    assert_eq!(code_of_the_order(&url), SafCode::UnsupportedOperation);
}

#[test]
fn an_unknown_verb_is_refused_as_unsupported_before_any_common_guard() {
    let url = an_order("afirma://unknownop?key=1234567&dat=file:/etc/hostname");

    assert_eq!(code_of_the_order(&url), SafCode::UnsupportedOperation);
}

#[test]
fn the_cipher_key_is_checked_before_the_rtservlet() {
    let url = a_signature_order("key=1234567&fileid=abc123&rtservlet=http://localhost/rt");

    assert_eq!(code_of_the_order(&url), SafCode::Params);
}

#[test]
fn the_rtservlet_is_checked_before_the_identifier() {
    let url = a_signature_order(&format!(
        "fileid={}&rtservlet=http://localhost/rt",
        "a".repeat(21)
    ));

    assert_eq!(code_of_the_order(&url), SafCode::LocalAccessBlocked);
}

#[test]
fn the_common_guards_are_checked_before_the_minimum_client_version() {
    let url = a_signature_order("dat=SG9sYQ&mcv=99.0.0&key=1234567");

    assert_eq!(code_of_the_order(&url), SafCode::Params);
}

#[test]
fn a_load_does_not_check_the_identifier_of_the_operation() {
    let url = an_order(&format!("afirma://load?id={}", "a".repeat(21)));

    read_operation(&url).expect("la carga no usa identificador");
}

#[test]
fn a_load_with_a_fileid_and_no_rtservlet_is_refused_as_a_parameter_error() {
    let url = an_order("afirma://load?fileid=abc123");

    assert_eq!(code_of_the_order(&url), SafCode::Params);
}

#[test]
fn a_valid_fileid_with_its_rtservlet_passes_the_common_guards() {
    let url = an_order(
        "afirma://sign?op=sign&format=CAdES&algorithm=SHA256withRSA&fileid=abc123\
         &rtservlet=https://sede.example/rt",
    );

    read_operation(&url).expect("el fileid y su rtservlet son validos");
}

#[test]
fn an_operation_the_original_does_not_know_is_shown_before_it_is_answered() {
    let url = AfirmaUrl::parse("afirma://noexiste?op=noexiste").expect("es del protocolo");

    let refusal = read_operation(&url).expect_err("no se atiende");

    assert!(refusal.is_shown_before_it_is_answered());
}

#[test]
fn a_multisignature_the_signer_does_not_support_is_shown_before_it_is_answered() {
    let refused_to_sign = [
        an_invoice_signature(COSIGN, "FacturaE"),
        an_invoice_signature(COUNTERSIGN, "FacturaE"),
        an_invoice_signature(COSIGN, AUTO),
        a_signature(COUNTERSIGN, ""),
        an_operation(&format!(
            "op={SIGN_AND_SAVE}&cop={COUNTERSIGN}&format=PAdES"
        )),
    ];

    for url in refused_to_sign {
        let refusal = read_operation(&url).expect_err("no se atiende");

        assert_eq!(refusal.code(), SafCode::UnsupportedOperation, "{url:?}");
        assert!(refusal.is_shown_before_it_is_answered(), "{url:?}");
    }
}

#[test]
fn an_operation_of_sign_and_save_it_does_not_know_is_answered_without_being_shown() {
    let url = an_operation(&format!("op={SIGN_AND_SAVE}&cop=resign&format=PAdES"));

    let refusal = read_operation(&url).expect_err("no se atiende");

    assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
    assert!(!refusal.is_shown_before_it_is_answered());
}

#[test]
fn a_verb_in_capitals_is_not_the_verb_of_the_original() {
    let refusal = refusal_of("afirma://SIGN?dat=ZmlybWFkbw");

    assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
}
