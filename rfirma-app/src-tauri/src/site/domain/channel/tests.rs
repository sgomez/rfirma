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

#[test]
fn the_relay_cipher_key_never_shows_up_in_a_debug_of_the_channel_location() {
    let key = super::super::protocol::CipherKey::from_url_parameter("12345678")
        .expect("longitud correcta")
        .expect("un valor no vacio siempre produce una clave");
    let info = super::super::protocol::RelayChannelInfo {
        operation: super::super::protocol::AfirmaUrl::parse(
            "afirma://sign?algorithm=SHA256withRSA",
        )
        .expect("la URL de operacion deberia parsear"),
        retrieve_servlet: None,
        store_servlet: "https://relay.example/store".to_owned(),
        id: "tx-1".to_owned(),
        fileid: None,
        key: Some(key),
        active_wait: false,
    };

    let printed = format!("{:?}", ChannelLocation::Relay(info));

    assert!(!printed.contains("12345678"));
}
