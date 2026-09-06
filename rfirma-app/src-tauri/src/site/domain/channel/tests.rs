use super::*;

#[test]
fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
    let error = ChannelError::new(Situation::NoDrawnPortIsFree, "Address already in use");

    assert_eq!(error.situation(), Situation::NoDrawnPortIsFree);
    assert_eq!(error.detail(), "Address already in use");
    assert!(error.to_string().contains("NoDrawnPortIsFree"));
    assert!(error.to_string().contains("Address already in use"));
}
