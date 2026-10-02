use super::*;

fn pairs(declared: &[(&str, &str)]) -> BTreeMap<String, String> {
    declared
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn without_config_the_signature_has_no_parameters() {
    assert_eq!(parameters_of(None), Ok(BTreeMap::new()));
}

#[test]
fn config_reads_one_key_per_line_and_skips_the_comments() {
    let parameters = parameters_of(Some(
        "# la política\nexpPolicy=FirmaAGE\n\nsignReason=Conforme\n",
    ));

    assert_eq!(
        parameters,
        Ok(pairs(&[
            ("expPolicy", "FirmaAGE"),
            ("signReason", "Conforme")
        ]))
    );
}

#[test]
fn config_breaks_the_line_at_the_escaped_line_break_the_console_leaves() {
    let parameters = parameters_of(Some(
        "#comentario\\nexpPolicy=FirmaAGE\\nsignatureProductionCity=Córdoba",
    ));

    assert_eq!(
        parameters,
        Ok(pairs(&[
            ("expPolicy", "FirmaAGE"),
            ("signatureProductionCity", "Córdoba")
        ]))
    );
}

#[test]
fn config_leaves_out_the_keys_the_launcher_interprets_as_a_site_would() {
    let parameters = parameters_of(Some(
        "headless=true\nmandatoryCertSelection=false\nprofile=baseline\nsignReason=Conforme",
    ));

    assert_eq!(parameters, Ok(pairs(&[("signReason", "Conforme")])));
}

#[test]
fn config_keeps_the_visible_box_as_it_comes() {
    let declared = "signaturePositionOnPageLowerLeftX=100\n\
                    signaturePositionOnPageLowerLeftY=100\n\
                    signaturePositionOnPageUpperRightX=300\n\
                    signaturePositionOnPageUpperRightY=200\n\
                    signaturePage=1";

    let parameters = parameters_of(Some(declared)).expect("el recuadro se acepta");

    assert_eq!(parameters.len(), 5);
    assert_eq!(
        parameters
            .get("signaturePositionOnPageUpperRightX")
            .map(String::as_str),
        Some("300")
    );
}

#[test]
fn config_refuses_an_appended_page_with_the_refusal_a_site_gets() {
    let declared = "signaturePositionOnPageLowerLeftX=100\n\
                    signaturePositionOnPageLowerLeftY=100\n\
                    signaturePositionOnPageUpperRightX=300\n\
                    signaturePositionOnPageUpperRightY=200\n\
                    signaturePage=append";
    let as_a_site = visible_signature_of(&pairs_of(declared).into_iter().collect())
        .expect_err("la sede tampoco lo acepta");

    assert_eq!(
        parameters_of(Some(declared)),
        Err(as_a_site.detail().to_owned())
    );
}

#[test]
fn config_refuses_a_box_to_mark_without_a_window_to_mark_it_in() {
    let refused = parameters_of(Some("visibleSignature=want")).expect_err("no hay ventana");

    assert!(refused.contains("visibleSignature=want"), "{refused}");
}

#[test]
fn config_with_an_optional_box_and_no_area_signs_invisible() {
    assert!(parameters_of(Some("visibleSignature=optional")).is_ok());
}
