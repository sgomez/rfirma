use super::*;

#[test]
fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
    let error = ChannelError::new(Situation::NoDrawnPortIsFree, "Address already in use");

    assert_eq!(error.situation(), Situation::NoDrawnPortIsFree);
    assert_eq!(error.detail(), "Address already in use");
    assert!(error.to_string().contains("NoDrawnPortIsFree"));
    assert!(error.to_string().contains("Address already in use"));
}

#[test]
fn a_relay_failure_carries_its_own_classified_refusal() {
    let refusal = super::super::protocol::Refusal::new(
        super::super::protocol::SafCode::RecoveringData,
        "el servlet de recuperacion no respondio",
    );
    let error = ChannelError::refused(refusal.clone());

    assert_eq!(error.situation(), Situation::Relay);
    assert_eq!(error.detail(), refusal.detail());
    assert_eq!(error.refusal(), Some(&refusal));
}

#[test]
fn a_plain_channel_failure_has_no_refusal_to_show() {
    let error = ChannelError::new(Situation::NotListening, "no se puede escuchar");

    assert_eq!(error.refusal(), None);
}
