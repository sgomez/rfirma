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

#[test]
fn the_batch_services_double_answers_what_it_was_configured_to_and_keeps_what_it_received() {
    use crate::site::application::tests::{InMemoryBatchServices, ReceivedBatchCall};
    use crate::site::domain::batch::{BatchFormat, TriSign, TriphaseData};

    let batch = InMemoryBatchServices::answering(b"presign-ok".to_vec(), b"postsign-ok".to_vec());
    let certs = vec![b"cert".to_vec()];
    let tridata = TriphaseData::new(
        None,
        vec![TriSign::new(Some("001".to_owned()), None, vec![])],
    );

    assert_eq!(
        batch
            .presign(
                "https://presigner.example",
                BatchFormat::Xml,
                "bG90ZQ",
                &certs
            )
            .expect("responde"),
        b"presign-ok"
    );
    assert_eq!(
        batch
            .postsign(
                "https://postsigner.example",
                BatchFormat::Xml,
                "bG90ZQ",
                &certs,
                &tridata
            )
            .expect("responde"),
        b"postsign-ok"
    );

    let received = batch.received();
    assert_eq!(received.len(), 2);
    match &received[0] {
        ReceivedBatchCall::Presign {
            url,
            format,
            lote_base64,
            certs: received_certs,
        } => {
            assert_eq!(url, "https://presigner.example");
            assert_eq!(*format, BatchFormat::Xml);
            assert_eq!(lote_base64, "bG90ZQ");
            assert_eq!(received_certs, &certs);
        }
        ReceivedBatchCall::Postsign { .. } => panic!("la primera llamada es la prefirma"),
    }
    match &received[1] {
        ReceivedBatchCall::Postsign {
            url,
            format,
            lote_base64,
            certs: received_certs,
            tridata: received_tridata,
        } => {
            assert_eq!(url, "https://postsigner.example");
            assert_eq!(*format, BatchFormat::Xml);
            assert_eq!(lote_base64, "bG90ZQ");
            assert_eq!(received_certs, &certs);
            assert_eq!(received_tridata, &tridata);
        }
        ReceivedBatchCall::Presign { .. } => panic!("la segunda llamada es la postfirma"),
    }
}

#[test]
fn an_unreachable_batch_service_fails_both_verbs() {
    use crate::site::application::tests::InMemoryBatchServices;
    use crate::site::domain::batch::{BatchFormat, TriSign, TriphaseData};

    let batch = InMemoryBatchServices::unreachable();
    let tridata = TriphaseData::new(
        None,
        vec![TriSign::new(Some("001".to_owned()), None, vec![])],
    );

    assert!(batch
        .presign("https://presigner.example", BatchFormat::Xml, "bG90ZQ", &[])
        .is_err());
    assert!(batch
        .postsign(
            "https://postsigner.example",
            BatchFormat::Xml,
            "bG90ZQ",
            &[],
            &tridata
        )
        .is_err());
}
