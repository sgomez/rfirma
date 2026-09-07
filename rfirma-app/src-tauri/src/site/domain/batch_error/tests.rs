use super::*;

#[test]
fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
    let error = BatchError::new(Situation::PresignerUnreachable, "connection refused");

    assert_eq!(error.situation(), Situation::PresignerUnreachable);
    assert_eq!(error.detail(), "connection refused");
    assert!(error.to_string().contains("PresignerUnreachable"));
    assert!(error.to_string().contains("connection refused"));
}
