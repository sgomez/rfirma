use super::*;
use crate::protocol::SafCode;

fn properties(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn the_expression_crosses_to_the_engine_literally() {
    let expression = "subject.contains:PEREZ;issuer.contains:FNMT";
    let filter = site_filter(&properties(&[("filters", expression)])).expect("es aceptable");

    assert_eq!(
        filter.declared(),
        [("filters".to_owned(), expression.to_owned())]
    );
    assert_eq!(
        filter.as_java_properties(),
        format!("filters={expression}\n")
    );
}

#[test]
fn the_first_of_the_three_spellings_wins() {
    let all_three = properties(&[
        ("filter", "dnie:true"),
        ("filters", "ssl:true"),
        ("filters.1", "sscd:true"),
    ]);

    assert_eq!(
        site_filter(&all_three).expect("es aceptable").declared(),
        [("filter".to_owned(), "dnie:true".to_owned())]
    );

    let without_the_first = properties(&[("filters", "ssl:true"), ("filters.1", "sscd:true")]);
    assert_eq!(
        site_filter(&without_the_first)
            .expect("es aceptable")
            .declared(),
        [("filters".to_owned(), "ssl:true".to_owned())]
    );
}

#[test]
fn the_numbered_ones_are_collected_in_order_and_stop_at_the_first_gap() {
    let with_a_gap = properties(&[
        ("filters.1", "subject.contains:UNO"),
        ("filters.2", "subject.contains:DOS"),
        ("filters.4", "subject.contains:CUATRO"),
    ]);

    let filter = site_filter(&with_a_gap).expect("es aceptable");

    assert_eq!(
        filter.declared(),
        [
            ("filters.1".to_owned(), "subject.contains:UNO".to_owned()),
            ("filters.2".to_owned(), "subject.contains:DOS".to_owned()),
        ]
    );
}

#[test]
fn a_site_that_declares_nothing_still_gets_the_engine_called() {
    let filter = site_filter(&properties(&[("format", "PAdES")])).expect("es aceptable");

    assert!(filter.declares_nothing());
    assert_eq!(filter.as_java_properties(), "");
}

#[test]
fn a_criterion_outside_the_whitelist_is_refused_instead_of_ignored() {
    let refusal = site_filter(&properties(&[(
        "filters",
        "subject.contains:PEREZ;inventado:loquesea",
    )]))
    .expect_err("'inventado:' no existe");

    assert_eq!(refusal.code(), SafCode::Params);
    assert!(refusal.detail().contains("inventado:loquesea"));
}

#[test]
fn every_criterion_the_original_understands_is_accepted() {
    for criterion in ACCEPTED_CRITERIA {
        let expression = format!("{criterion}loquesea");
        assert!(
            site_filter(&properties(&[("filters", &expression)])).is_ok(),
            "«{expression}» tendria que cruzar al motor"
        );
    }

    assert!(site_filter(&properties(&[(
        "filters",
        "keyusage.digitalsignature:true"
    )]))
    .is_ok());
    assert!(site_filter(&properties(&[("filters", SATISFIED_BY_CONSTRUCTION)])).is_ok());
}

#[test]
fn the_criteria_are_recognised_regardless_of_case() {
    assert!(site_filter(&properties(&[("filters", "Subject.Contains:PEREZ")])).is_ok());
    assert!(site_filter(&properties(&[("filters", "NONEXPIRED:true")])).is_ok());
}

#[test]
fn the_four_unmeasured_criteria_are_accepted_all_the_same() {
    for criterion in UNMEASURED_CRITERIA {
        assert!(
            ACCEPTED_CRITERIA.contains(criterion),
            "«{criterion}» esta anotado como sin medir pero no cruza"
        );
        assert!(site_filter(&properties(&[("filters", &format!("{criterion}true"))])).is_ok());
    }
}

#[test]
fn the_sibling_keys_are_not_criteria_and_do_not_trip_the_whitelist() {
    let with_siblings = properties(&[
        ("headless", "true"),
        ("mandatoryCertSelection", "false"),
        ("filters", "subject.contains:PEREZ"),
    ]);

    assert!(site_filter(&with_siblings).is_ok());
}

#[test]
fn a_value_with_backslashes_survives_the_properties_block() {
    let expression = r"subject.rfc2254:(cn=PEREZ\, JUAN)";
    let filter = site_filter(&properties(&[("filters", expression)])).expect("es aceptable");

    assert_eq!(
        filter.as_java_properties(),
        "filters=subject.rfc2254:(cn=PEREZ\\\\, JUAN)\n"
    );
}

#[test]
fn a_value_with_accents_reaches_the_engine_unchanged() {
    let expression = "subject.contains:MUÑOZ PÉREZ";
    let filter = site_filter(&properties(&[("filters", expression)])).expect("es aceptable");

    let block = filter.as_java_properties();

    assert_eq!(block, "filters=subject.contains:MUÑOZ PÉREZ\n");
    assert!(block.contains('Ñ'));
    assert!(block.contains('É'));
}

#[test]
fn a_newline_inside_a_value_cannot_split_the_block() {
    let filter = site_filter(&properties(&[("filters", "subject.contains:A\nB")]))
        .expect("es aceptable");

    assert_eq!(
        filter.as_java_properties(),
        "filters=subject.contains:A\\nB\n"
    );
    assert_eq!(filter.as_java_properties().lines().count(), 1);
}
