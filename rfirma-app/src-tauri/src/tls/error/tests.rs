use super::*;

#[test]
fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
    let error = TlsError::new(Situation::MaterialDamaged, "PEM routines::no start line");

    assert_eq!(error.situation(), Situation::MaterialDamaged);
    assert_eq!(error.detail(), "PEM routines::no start line");
    assert!(error.to_string().contains("MaterialDamaged"));
    assert!(error.to_string().contains("no start line"));
}
