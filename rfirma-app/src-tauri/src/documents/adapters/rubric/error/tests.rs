use super::*;

#[test]
fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
    let error = RubricError::new(Situation::DamagedImage, "invalid JPEG marker");

    assert_eq!(error.situation(), Situation::DamagedImage);
    assert_eq!(error.detail(), "invalid JPEG marker");
    assert!(error.to_string().contains("DamagedImage"));
    assert!(error.to_string().contains("invalid JPEG marker"));
}
