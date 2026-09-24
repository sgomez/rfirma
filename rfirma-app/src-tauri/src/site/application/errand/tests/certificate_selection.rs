//! Pruebas de la seleccion de certificado de sede, del arranque a la respuesta.

use std::cell::RefCell;
use std::sync::Arc;

use super::support::*;
use crate::crossing::Failure;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::certificates::ListedCertificates;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::signing::application::tests::a_memory;
use crate::site::adapters::frontier;
use crate::site::application::errand::*;
use crate::site::application::site::{attend_launch, Attendance};
use crate::site::domain::channel::{
    ArrivalMode, ChannelDuty, ChannelLocation, OpenChannel, Shutdown,
};
use crate::site::domain::protocol::{
    NegotiatedCredential, SafCode, THE_PORT_OF_THE_THIRD_PROTOCOL,
};
use base64::Engine as _;
use std::time::Duration;

#[test]
#[expect(clippy::too_many_lines)]
fn the_three_verbs_run_the_errand_with_a_codec_a_filter_and_a_transport_in_memory() {
    let opened = RefCell::new(Vec::new());
    let transport = |location: &ChannelLocation, duty: ChannelDuty| {
        let ChannelLocation::Drawn(ports) = location else {
            panic!("esta prueba sortea puertos: {location:?}");
        };
        opened.borrow_mut().push((ports[0], duty));
        Ok(OpenChannel::new(ports[0], Shutdown::of(|| {})))
    };
    let live = LiveErrand::default();

    let _channel = Transport::open(
        &transport,
        &ChannelLocation::Drawn(vec![54001]),
        ChannelDuty::Serve(NegotiatedCredential::Required(a_credential())),
    )
    .expect("el transporte en memoria abre");
    let codec: NegotiatedCodec = Arc::new(ACodec::answering(Vec::new()));
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        Arc::clone(&codec)
    )));
    assert_eq!(opened.borrow().len(), 1, "un canal, y por el puerto pedido");

    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened_documents = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened_documents,
        &memory,
        &scratch,
    );

    let (handle, mut wire) = the_wire();
    let step = attend(&desk, an_operation(""), handle, &live).expect("hay codec negociado");
    let ErrandStep::Answering(SiteOutcome::Refused(refusal)) = &step else {
        panic!("sin almacen la mesa contesta en el acto: {step:?}");
    };
    let (failure, code) = frontier::told(refusal);
    assert_eq!(code, SafCode::CannotFindKeystore);
    assert_eq!(failure.situation, "moduleNotFound");
    assert!(
        what_the_site_received(&mut wire)
            .is_some_and(|line| line.starts_with("Refused(Token(TokenError")),
        "lo que sale lo escribe el codec negociado, no el tramite"
    );
    assert!(
        step.moment().is_none(),
        "lo contestado no es un momento nuevo"
    );
    assert!(live.current().is_none(), "y el tramite ha terminado");
    assert!(
        live.the_request().is_none(),
        "sin peticion que volver a atender"
    );
    assert!(
        live.codec().is_some(),
        "el codec negociado sobrevive al tramite: el canal sigue en pie"
    );

    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        codec
    )));
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);

    assert!(look_again(&desk, &live).is_none());

    let refused = consent(&desk, "cualquiera", &live).expect_err("no hay nada consentible");
    assert_eq!(Failure::from(refused).situation, "siteErrandNotLive");
    assert!(what_the_site_received(&mut wire).is_none());
    assert!(live.current().is_some(), "y el tramite sigue vivo");

    let outcome = decline(&live);
    assert!(matches!(outcome, SiteOutcome::Cancelled));
    assert_eq!(
        what_the_site_received(&mut wire).as_deref(),
        Some("Cancelled"),
        "lo que sale lo escribe el codec negociado, no el tramite"
    );
    assert!(live.current().is_none(), "el tramite ha terminado");
    assert!(
        consent(&desk, "cualquiera", &live).is_err(),
        "y no queda nada que consentir"
    );
}
#[test]
fn what_the_codec_does_not_attend_is_answered_with_the_codec_s_own_line() {
    let live = LiveErrand::speaking(Arc::new(ACodec::answering(vec![SiteRequest::NotAttended(
        crate::site::domain::protocol::Refusal::new(SafCode::UnsupportedFormat, "eso no"),
    )])));
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );

    let (handle, mut wire) = the_wire();
    let step = attend(&desk, an_operation(""), handle, &live).expect("hay codec");

    assert!(matches!(
        step,
        ErrandStep::Answering(SiteOutcome::RefusedByTheProtocol(_))
    ));
    assert!(
        step.moment().is_none(),
        "lo contestado no es un momento nuevo"
    );
    assert!(live.moment().is_none());
    assert!(
        what_the_site_received(&mut wire)
            .is_some_and(|line| line.starts_with("RefusedByTheProtocol(")),
        "la linea la escribe el codec"
    );
    assert!(
        live.the_request().is_none(),
        "lo que no se atiende no se apunta"
    );
}
/// El mismo trámite de selección de certificado, de punta a punta, sobre la forma de arranque
/// que se le pase: puertos sorteados (protocolo 4) o puerto fijo (protocolo 3).
#[expect(clippy::too_many_lines)]
fn a_selection_of_a_certificate_goes_all_the_way_from_the_launch_to_the_answer_over(
    launch: &str,
    expected_port: u16,
) {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let asked = RefCell::new(Vec::new());
    let engine = AnEngine::answering(&[&[0], &[0]]);

    let attendance = attend_launch(launch, &a_codec_table(), &a_transport(&asked), &live);
    let Attendance::Serving { channel, .. } = &attendance else {
        panic!("la invocacion es buena: {attendance:?}");
    };
    assert_eq!(
        channel.port(),
        expected_port,
        "el tramite escucha en el puerto que declara el protocolo"
    );
    assert!(
        live.current().is_some(),
        "el tramite queda vivo mientras se atiende"
    );
    live.browser_arrived();

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let url = arriving_over_the_channel(&format!(
        "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"
    ));
    let request = requested(&url);
    let step = consent_for(
        &engine,
        &request,
        ours.clone(),
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
        &live,
    );
    let ErrandStep::AskingForConsent {
        certificates: rows, ..
    } = step
    else {
        panic!("hay un certificado que la sede acepta: {step:?}");
    };
    assert_eq!(rows.len(), 1);
    assert!(
        live.current().is_some(),
        "consintiendo, el tramite sigue vivo"
    );
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "el momento del consentimiento no escribe nada en el cable"
    );

    let reply = identity_handed_over(
        &engine,
        request.filter(),
        false,
        &ours,
        &rows[0].id,
        &crate::site::application::tests::Directory {
            certificates: ours.clone(),
            listed: &listed,
            memory: &memory,
        },
        &live,
    );
    let SiteOutcome::Certificate(der) = &reply else {
        panic!("la persona se ha identificado: {reply:?}");
    };
    assert_eq!(
        der,
        ours[0].der(),
        "lo que la persona entrego es su DER, sin envolver"
    );
    let encoded = base64::engine::general_purpose::URL_SAFE.encode(ours[0].der());
    assert_eq!(on_the_wire(&reply), encoded);
    assert_eq!(
        what_the_site_received(&mut wire),
        Some(encoded.clone()),
        "la sede recibe el certificado en el acto, por el asa del tramite"
    );
    assert!(
        live.current().is_some() && live.the_request().is_none(),
        "contestada la sede, el WebSocket espera la siguiente operacion sin nada pendiente"
    );
}
#[test]
fn a_selection_of_a_certificate_goes_all_the_way_from_the_launch_to_the_answer() {
    a_selection_of_a_certificate_goes_all_the_way_from_the_launch_to_the_answer_over(
        &a_launch("54001,54002,54003"),
        54001,
    );
}
#[test]
fn a_selection_of_a_certificate_over_the_third_protocol_goes_all_the_way_from_the_launch_to_the_answer(
) {
    a_selection_of_a_certificate_goes_all_the_way_from_the_launch_to_the_answer_over(
        &a_v3_launch(),
        THE_PORT_OF_THE_THIRD_PROTOCOL,
    );
}
#[test]
fn a_selection_that_is_declined_ends_in_a_cancel_on_the_wire_and_nothing_after_it() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let asked = RefCell::new(Vec::new());
    let engine = AnEngine::answering(&[&[0]]);

    let attendance = attend_launch(
        &a_launch("54001,54002,54003"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );
    assert!(
        matches!(attendance, Attendance::Serving { .. }),
        "la invocacion es buena: {attendance:?}"
    );

    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let url = arriving_over_the_channel(&format!(
        "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"
    ));
    let step = consent_for(
        &engine,
        &requested(&url),
        ours,
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
        &live,
    );
    assert!(
        matches!(step, ErrandStep::AskingForConsent { .. }),
        "hay algo que consentir: {step:?}"
    );
    assert_eq!(what_the_site_received(&mut wire), None);

    let reply = declined(&live);

    assert!(matches!(reply, SiteOutcome::Cancelled), "{reply:?}");
    assert_eq!(
        what_the_site_received(&mut wire),
        Some("CANCEL".to_owned()),
        "cancelar sale al cable en el acto, sin esperar a que nadie cierre nada"
    );
    assert!(live.what_the_site_asked().is_none());

    declined(&live);
    assert_eq!(what_the_site_received(&mut wire), None);
}
#[test]
fn closing_the_window_with_an_errand_still_alive_cancels_it_on_the_wire() {
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));

    answer_before_closing_within(&live, Duration::from_secs(1));

    assert_eq!(what_the_site_received(&mut wire), Some("CANCEL".to_owned()));
    assert!(live.current().is_none());
}
#[test]
fn closing_the_window_with_the_outcome_already_on_screen_sends_nothing() {
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));
    declined(&live);
    assert_eq!(what_the_site_received(&mut wire), Some("CANCEL".to_owned()));

    answer_before_closing_within(&live, Duration::from_secs(1));

    assert_eq!(what_the_site_received(&mut wire), None);
}
#[test]
fn closing_the_window_gives_up_waiting_for_the_acknowledgement_past_its_threshold() {
    let live = a_live();
    let (handle, mut wire) = a_wire_that_never_confirms();
    live.answer_through(handle);
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));

    let started = std::time::Instant::now();
    answer_before_closing_within(&live, Duration::from_millis(20));
    let elapsed = started.elapsed();

    assert_eq!(what_the_site_received(&mut wire), Some("CANCEL".to_owned()));
    assert!(
        elapsed < Duration::from_secs(1),
        "no debe esperar mas alla de su tope: {elapsed:?}"
    );
}
#[test]
fn a_connection_that_drops_while_the_operation_is_pending_does_not_take_the_errand_down() {
    let live = a_live();
    let (handle, wire) = the_wire();
    live.answer_through(handle);
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));

    drop(wire);

    let reply = declined(&live);

    assert!(
        matches!(reply, SiteOutcome::Cancelled),
        "el desenlace es el mismo, lo lea alguien o no: {reply:?}"
    );
    assert!(
        live.current().is_none(),
        "y el tramite termina igual, sin reintentar nada"
    );
}
