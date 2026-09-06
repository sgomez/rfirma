use super::*;

#[test]
fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
    let error = DesktopError::new(Situation::TheListIsNotWritable, "Permission denied");

    assert_eq!(error.situation(), Situation::TheListIsNotWritable);
    assert_eq!(error.detail(), "Permission denied");
    assert!(error.to_string().contains("TheListIsNotWritable"));
    assert!(error.to_string().contains("Permission denied"));
}
