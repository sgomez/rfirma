use super::*;

#[test]
fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
    let error = RelayError::new(Situation::ServletUnreachable, "connection refused");

    assert_eq!(error.situation(), Situation::ServletUnreachable);
    assert_eq!(error.detail(), "connection refused");
    assert!(error.to_string().contains("ServletUnreachable"));
    assert!(error.to_string().contains("connection refused"));
}
