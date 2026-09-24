use super::super::*;
use super::fixtures::{
    a_countersignature, a_signature, an_operation, dat, properties, read_operation,
};
use crate::site::domain::protocol::XadesEnvelope;
use crate::site::domain::triphase_server::ServerFormat;

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

#[test]
fn signing_with_sha1_is_attended_like_the_original_attends_it() {
    let url = an_operation(&format!(
        "op={SIGN}&format=PAdES&algorithm=SHA1withRSA&dat={}",
        dat(b"%PDF-1.7\n")
    ));

    let SiteOperation::Sign(request) = read_operation(&url).expect("se atiende") else {
        panic!("es una firma");
    };
    assert_eq!(request.algorithm(), AskedAlgorithm::Sha1);
}

#[test]
fn each_missing_parameter_of_a_signature_names_itself() {
    for (parameters, blamed) in [
        ("op=sign", Parameter::Format),
        ("op=sign&format=PAdES", Parameter::Algorithm),
    ] {
        let refusal = read_operation(&an_operation(parameters)).expect_err("falta uno");

        assert_eq!(refusal.code(), SafCode::Params);
        assert_eq!(refusal.blame(), Some(blamed), "en «{parameters}»");
    }
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
fn a_countersignature_under_format_auto_over_a_cades_signature_is_a_cades_one() {
    let cades = include_bytes!("../../../../../../../../testdata/reference/cades-implicit.p7s");
    let url = an_operation(&format!(
        "op={COUNTERSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format={AUTO}&algorithm=SHA256withRSA&dat={}",
        dat(cades)
    ));

    let SiteOperation::Sign(request) = read_operation(&url).expect("una firma CAdES es CAdES")
    else {
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
fn a_countersignature_under_format_auto_over_a_signed_xml_is_a_xades_one() {
    let xml = include_bytes!("../../../../../../../../testdata/reference/xades-enveloping.xml");
    let url = an_operation(&format!(
        "op={COUNTERSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format={AUTO}&\
         algorithm=SHA256withRSA&dat={}",
        dat(xml)
    ));

    let SiteOperation::Sign(request) = read_operation(&url).expect("un XML firmado es XAdES")
    else {
        panic!("es una firma");
    };

    assert_eq!(
        request.format(),
        RequestedFormat::Xades(XadesEnvelope::Enveloping)
    );
}

#[test]
fn a_signature_without_data_asks_the_person_for_the_document() {
    let url =
        an_operation("op=sign&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&algorithm=SHA256withRSA");

    let operation = read_operation(&url).expect("el original lo atiende");

    assert!(
        matches!(operation, SiteOperation::SignWithoutDocument(_)),
        "sin 'dat' el documento lo elige la persona"
    );
}

#[test]
fn a_cosignature_and_a_countersignature_without_data_ask_for_it_too() {
    for verb in [COSIGN, COUNTERSIGN] {
        let url = an_operation(&format!(
            "op={verb}&idsession=8jAkPZfRw2mQxN4TbYuL&format=CAdES&algorithm=SHA256withRSA"
        ));

        let operation = read_operation(&url).expect("el original lo atiende");

        assert!(
            matches!(operation, SiteOperation::SignWithoutDocument(_)),
            "'{verb}' sin 'dat' tampoco es un rechazo"
        );
    }
}

#[test]
fn the_chosen_document_resolves_the_format_auto_of_a_signature_without_data() {
    let url =
        an_operation("op=sign&idsession=8jAkPZfRw2mQxN4TbYuL&format=auto&algorithm=SHA256withRSA");

    let SiteOperation::SignWithoutDocument(pending) = read_operation(&url).expect("se atiende")
    else {
        panic!("sin 'dat' se pide el documento");
    };
    let request = pending.with_chosen_document(b"%PDF-1.7\nelegido".to_vec());

    assert_eq!(request.format(), RequestedFormat::Pades);
    assert_eq!(request.document(), b"%PDF-1.7\nelegido");
    assert_eq!(request.round(), SignatureRound::First);
}

#[test]
fn a_cades_triphase_signature_goes_through_the_site_server() {
    let url = an_operation(&format!(
        "op={COSIGN}&format=CAdEStri&algorithm=SHA256&dat={}",
        dat(b"firma previa")
    ));

    let SiteOperation::Sign(request) = read_operation(&url).expect("se atiende") else {
        panic!("es una firma");
    };
    assert_eq!(request.through_the_site_server(), Some(ServerFormat::Cades));
    assert_eq!(request.format(), RequestedFormat::Cades);
}

#[test]
fn a_plain_cades_signature_is_made_here() {
    let url = an_operation(&format!(
        "op={SIGN}&format=CAdES&algorithm=SHA256&dat={}",
        dat(b"datos")
    ));

    let SiteOperation::Sign(request) = read_operation(&url).expect("se atiende") else {
        panic!("es una firma");
    };
    assert_eq!(request.through_the_site_server(), None);
}

#[test]
fn a_cades_triphase_signature_without_data_still_goes_through_the_site_server() {
    let url = an_operation("op=sign&format=CAdEStri&algorithm=SHA256");

    let SiteOperation::SignWithoutDocument(pending) = read_operation(&url).expect("se atiende")
    else {
        panic!("es una firma sin documento");
    };
    assert_eq!(
        pending
            .with_chosen_document(b"datos".to_vec())
            .through_the_site_server(),
        Some(ServerFormat::Cades)
    );
}

#[test]
fn a_pades_triphase_signature_goes_through_the_site_server() {
    let url = an_operation(&format!(
        "op={SIGN}&format=PAdEStri&algorithm=SHA256&dat={}",
        dat(b"%PDF-1.7\n")
    ));

    let SiteOperation::Sign(request) = read_operation(&url).expect("se atiende") else {
        panic!("es una firma");
    };
    assert_eq!(request.through_the_site_server(), Some(ServerFormat::Pades));
    assert_eq!(request.format(), RequestedFormat::Pades);
}
