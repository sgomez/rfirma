use super::*;

/// **Grada A**: la traducción del vocabulario del protocolo al del puente, entera.
#[test]
fn every_format_the_site_can_name_crosses_to_the_one_the_bridge_knows() {
    let table = [
        (RequestedFormat::Pades, Format::Pades),
        (RequestedFormat::Cades, Format::Cades),
        (RequestedFormat::CadesAsicS, Format::CadesAsicS),
        (RequestedFormat::Cms, Format::Cms),
        (
            RequestedFormat::Xades(XadesEnvelope::Detached),
            Format::Xades(XadesVariant::Detached),
        ),
        (
            RequestedFormat::Xades(XadesEnvelope::Enveloping),
            Format::Xades(XadesVariant::Enveloping),
        ),
        (
            RequestedFormat::Xades(XadesEnvelope::Enveloped),
            Format::Xades(XadesVariant::Enveloped),
        ),
        (
            RequestedFormat::Xades(XadesEnvelope::AsicS),
            Format::Xades(XadesVariant::AsicS),
        ),
        (RequestedFormat::FacturaE, Format::FacturaE),
    ];

    assert_eq!(table.len(), Format::ALL.len(), "sin formato sin traducir");
    for (requested, expected) in table {
        assert_eq!(Format::from(requested), expected, "{requested:?}");
    }
}

#[test]
fn what_arrives_at_the_inbox_notifies_arrival_and_delivers_operations() {
    let arrived = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let delivered = std::sync::Arc::new(std::sync::Mutex::new(None));

    let arrived_clone = std::sync::Arc::clone(&arrived);
    let delivered_clone = std::sync::Arc::clone(&delivered);
    let inbox = Inbox::of(
        move || {
            arrived_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        },
        move |url, _reply| {
            *delivered_clone.lock().expect("el candado") = Some(url);
        },
    );

    assert!(!arrived.load(std::sync::atomic::Ordering::SeqCst));
    inbox.arrived();
    assert!(arrived.load(std::sync::atomic::Ordering::SeqCst));

    let url = AfirmaUrl::parse("afirma://websocket?ports=51001,51002,51003&v=4").expect("url");
    inbox.deliver(url.clone(), ReplyHandle::of(|_| {}));
    assert_eq!(delivered.lock().expect("el candado").as_ref(), Some(&url));
}

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

#[test]
fn the_token_double_counts_the_secrets_asked_and_keeps_what_it_signed() {
    use crate::identity::application::tests::a_certificate;
    use crate::site::application::tests::InMemoryTokenSigning;

    let token = InMemoryTokenSigning::default();
    let certificate = a_certificate("FNMT-ACTIVO", b"der");

    token.secret_of(&certificate).expect("el secreto sale");
    for pre in [b"uno".as_slice(), b"dos".as_slice()] {
        assert_eq!(
            token
                .sign(&certificate, "1234", "SHA256", pre)
                .expect("firma"),
            [b"PK1:".as_slice(), pre].concat()
        );
    }

    assert_eq!(token.secrets_asked(), 1);
    assert_eq!(
        token.signed(),
        vec![
            ("SHA256".to_owned(), b"uno".to_vec()),
            ("SHA256".to_owned(), b"dos".to_vec()),
        ]
    );
}

#[test]
fn a_token_double_that_refuses_signs_nothing() {
    use crate::identity::application::tests::a_certificate;
    use crate::site::application::tests::InMemoryTokenSigning;
    use crate::site::domain::protocol::SafCode;
    use crate::site::domain::signing::SigningRefusal;

    let token = InMemoryTokenSigning::refusing(SigningRefusal {
        code: SafCode::CannotAccessKeystore,
        situation: "incorrectPin".to_owned(),
        detail: "CKR_PIN_INCORRECT".to_owned(),
        attempts_left: None,
    });
    let certificate = a_certificate("FNMT-ACTIVO", b"der");

    assert!(token.secret_of(&certificate).is_err());
    assert!(token.sign(&certificate, "1234", "SHA256", b"uno").is_err());
    assert!(token.signed().is_empty());
}
