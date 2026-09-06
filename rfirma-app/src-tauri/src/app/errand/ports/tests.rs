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
    let transport = |ports: &[u16], _duty: ChannelDuty| {
        Ok(OpenChannel::new(
            ports[0],
            crate::channel::Shutdown::of(|| {}),
        ))
    };
    let opened = Transport::open(
        &transport,
        &[51001],
        ChannelDuty::Refuse(crate::protocol::WireAnswer::refused(
            crate::protocol::SafCode::CannotOpenSocket,
        )),
    )
    .expect("abre");
    assert_eq!(opened.port(), 51001);
}
