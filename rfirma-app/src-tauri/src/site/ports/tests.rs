use super::*;

#[test]
fn what_is_answered_is_what_the_other_end_receives() {
    let received = std::sync::Arc::new(std::sync::Mutex::new(None));
    let keeping = std::sync::Arc::clone(&received);
    let handle = ReplyHandle::of(move |text| {
        *keeping.lock().expect("el candado") = Some(text);
    });

    handle.answer("OK".to_owned());

    assert_eq!(received.lock().expect("el candado").as_deref(), Some("OK"));
}

#[test]
fn a_closure_with_the_right_shape_is_a_transport() {
    let transport = |location: &ChannelLocation, _duty: ChannelDuty| {
        let ChannelLocation::Drawn(ports) = location else {
            panic!("esta prueba solo sortea puertos");
        };
        Ok(OpenChannel::new(
            ports[0],
            crate::site::domain::channel::Shutdown::of(|| {}),
        ))
    };
    let opened = Transport::open(
        &transport,
        &ChannelLocation::Drawn(vec![51001]),
        ChannelDuty::Refuse(crate::site::domain::protocol::WireAnswer::refused(
            crate::site::domain::protocol::SafCode::CannotOpenSocket,
        )),
    )
    .expect("abre");
    assert_eq!(opened.port(), 51001);
}

#[test]
fn stored_data_comes_back_from_a_retrieve_by_the_same_id() {
    let servlets = crate::site::application::tests::InMemoryServlets::default();

    servlets
        .store("https://servlet.example/store", "tx-1", "0.dato")
        .expect("almacena");

    assert_eq!(
        servlets
            .retrieve("https://servlet.example/retrieve", "tx-1")
            .expect("recupera"),
        "0.dato"
    );
}

#[test]
fn waiting_is_recorded_by_id_without_touching_what_is_stored() {
    let servlets = crate::site::application::tests::InMemoryServlets::default();

    servlets
        .wait("https://servlet.example/store", "tx-1")
        .expect("espera");

    assert_eq!(servlets.waited_ids(), vec!["tx-1".to_owned()]);
}

#[test]
fn an_unreachable_servlet_fails_every_verb() {
    let servlets = crate::site::application::tests::InMemoryServlets::unreachable();

    assert!(servlets
        .retrieve("https://servlet.example/retrieve", "tx-1")
        .is_err());
    assert!(servlets
        .store("https://servlet.example/store", "tx-1", "0.dato")
        .is_err());
    assert!(servlets
        .wait("https://servlet.example/store", "tx-1")
        .is_err());
}
