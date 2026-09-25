//! Pruebas del token, el almacen vacio y el arranque de un segundo tramite.

use std::cell::RefCell;

use super::support::*;
use super::support_requests::*;
use crate::crossing::Failure;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::certificates::ListedCertificates;
use crate::identity::application::tests::{
    a_usable_certificate, an_expired_certificate, listed_from, NoMemory,
};
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::application::tests::a_memory;
use crate::site::application::errand::*;
use crate::site::application::site::{attend_launch, Attendance};
use crate::site::domain::channel::{
    ArrivalMode, ChannelDuty, ChannelLocation, OpenChannel, Shutdown,
};
use crate::site::domain::protocol::{
    NegotiatedCredential, Parameter, SafCode, SiteFilter, WireAnswer,
};

#[test]
fn a_refusal_of_the_protocol_never_reaches_the_token() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let live = a_live();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let step = attend_operation(
        &a_desk(
            &engine,
            &policies,
            &[],
            home.path(),
            &listed,
            &opened,
            &memory,
            home.path(),
        ),
        &an_operation("&dat=file:///etc/shadow"),
        decoded(&an_operation("&dat=file:///etc/shadow")),
        &live,
    );

    let ErrandStep::ShowingTheRefusal(refusal) = step else {
        panic!("el protocolo rechaza la lectura de un fichero local: {step:?}");
    };
    assert_eq!(
        refusal.answer().on_the_wire(),
        WireAnswer::refused_because_of(SafCode::Params, Parameter::Data).on_the_wire()
    );
}
#[test]
fn a_token_that_cannot_be_listed_answers_with_the_code_of_its_own_situation() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let live = a_live();

    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let step = attend_operation(
        &a_desk(
            &engine,
            &policies,
            &[],
            home.path(),
            &listed,
            &opened,
            &memory,
            home.path(),
        ),
        &an_operation(""),
        decoded(&an_operation("")),
        &live,
    );

    let ErrandStep::Answering(reply) = step else {
        panic!("no hay almacenes: {step:?}");
    };
    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(crate::identity::adapters::failures::code_of_token(
            crate::identity::domain::error::Situation::ModuleNotFound
        ))
        .on_the_wire()
    );
}
#[test]
fn the_person_saying_no_is_the_only_cancellation() {
    let live = a_live();
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));

    let reply = declined(&live);

    assert_eq!(on_the_wire(&reply), "CANCEL");
    assert!(live.current().is_none(), "cancelado, el tramite se acaba");
}
#[test]
fn a_second_launch_is_refused_while_the_first_errand_is_live() {
    let live = a_live();
    let asked = RefCell::new(Vec::new());

    let first = attend_launch(
        &a_launch("54001"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );
    assert!(matches!(first, Attendance::Serving { .. }), "{first:?}");

    let second = attend_launch(
        &a_launch("55001"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );
    let Attendance::RefusingOverTheChannel { answer, .. } = second else {
        panic!("el segundo se rechaza por su socket: {second:?}");
    };
    assert_eq!(
        answer.on_the_wire(),
        WireAnswer::refused(SafCode::CannotOpenSocket).on_the_wire()
    );

    let errand = live.current().expect("el primer tramite sigue vivo");
    assert_eq!(errand.arrival(), ArrivalMode::Awaited);
}
#[test]
fn a_launch_that_loses_the_place_while_its_channel_opens_has_it_closed_and_is_refused() {
    use std::cell::Cell;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let live = a_live();
    let closed = Arc::new(AtomicBool::new(false));
    let opened = Cell::new(0_u8);

    let transport = |location: &ChannelLocation, _duty: ChannelDuty| {
        let ChannelLocation::Drawn(ports) = location else {
            panic!("esta prueba sortea puertos: {location:?}");
        };
        opened.set(opened.get() + 1);
        if opened.get() == 1 {
            assert!(live.begin(Errand::of(
                NegotiatedCredential::Required(a_credential()),
                ArrivalMode::Awaited,
                a_codec()
            )));
            let closed = Arc::clone(&closed);
            return Ok(OpenChannel::new(
                ports[0],
                Shutdown::of(move || closed.store(true, Ordering::SeqCst)),
            ));
        }
        Ok(OpenChannel::new(ports[0], Shutdown::of(|| {})))
    };

    let attendance = attend_launch(
        &a_launch("55001,55002"),
        &a_codec_table(),
        &transport,
        &live,
    );

    let Attendance::RefusingOverTheChannel { answer, .. } = attendance else {
        panic!("la que llega tarde se rechaza por su socket: {attendance:?}");
    };
    assert_eq!(
        answer.on_the_wire(),
        WireAnswer::refused(SafCode::CannotOpenSocket).on_the_wire()
    );
    assert!(
        closed.load(Ordering::SeqCst),
        "el canal de la que llega tarde deja de escuchar: soltarlo sin llamar al asa no lo cierra"
    );

    let errand = live
        .current()
        .expect("el tramite de la otra sede sigue vivo");
    assert_eq!(errand.arrival(), ArrivalMode::Awaited);
}
#[test]
fn once_the_first_client_has_left_the_next_launch_is_attended() {
    let live = a_live();
    let asked = RefCell::new(Vec::new());

    attend_launch(
        &a_launch("54001"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );
    declined(&live);
    live.the_first_client_left();

    let next = attend_launch(
        &a_launch("55001"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );

    assert!(matches!(next, Attendance::Serving { .. }), "{next:?}");
}
#[test]
fn the_live_errand_remembers_the_credential_and_the_arrival_and_nothing_else() {
    let live = a_live();
    let asked = RefCell::new(Vec::new());

    attend_launch(
        &a_launch("54001"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );

    let errand = live.current().expect("hay tramite vivo");
    assert_eq!(
        errand.credential(),
        &NegotiatedCredential::Required(a_credential())
    );
    assert_eq!(errand.arrival(), ArrivalMode::Awaited);
}
#[test]
fn a_certificate_the_site_no_longer_accepts_is_never_handed_over() {
    let ours: Vec<TokenCertificate> = vec![a_usable_certificate("FIRMA")];
    let (listed, handles) = listed_from(&ours);
    let live = a_live();

    let reply = identity_handed_over(
        &AnEngine::answering(&[&[]]),
        &SiteFilter::default(),
        false,
        &ours,
        &handles[0],
        &crate::site::application::tests::Directory {
            certificates: ours.clone(),
            listed: &listed,
            memory: &NoMemory,
        },
        &live,
    );

    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(SafCode::NoCertificatesInKeystore).on_the_wire()
    );
    assert!(
        reply
            .refusal()
            .is_some_and(|it| Failure::from(it).situation == "certificateNotFound"),
        "la ventana sabe cual es la situacion: {reply:?}"
    );
}
#[test]
fn an_expired_certificate_the_site_admits_is_never_handed_over() {
    let ours: Vec<TokenCertificate> = vec![an_expired_certificate("CADUCADO")];
    let (listed, handles) = listed_from(&ours);
    let live = a_live();

    let reply = identity_handed_over(
        &AnEngine::answering(&[&[0]]),
        &SiteFilter::default(),
        false,
        &ours,
        &handles[0],
        &crate::site::application::tests::Directory {
            certificates: ours.clone(),
            listed: &listed,
            memory: &NoMemory,
        },
        &live,
    );

    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(SafCode::NoCertificatesInKeystore).on_the_wire(),
        "aunque el motor lo admita, un certificado caducado no se entrega"
    );
    assert!(
        reply
            .refusal()
            .is_some_and(|it| Failure::from(it).situation == "certificateNotFound"),
        "la ventana sabe cual es la situacion: {reply:?}"
    );
}
#[test]
fn with_no_certificate_at_all_nothing_goes_out_and_the_errand_stays_live() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let engine = AnEngine::answering(&[]);

    let live = a_live();
    assert!(
        live.begin(Errand::of(
            NegotiatedCredential::Required(a_credential()),
            ArrivalMode::Awaited,
            a_codec()
        )),
        "la plaza es suya"
    );
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let url = arriving_over_the_channel(&format!(
        "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"
    ));

    live.keep_the_request(url.clone());
    let step = consent_for(
        &engine,
        &requested(&url),
        Vec::new(),
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
        &live,
    );

    assert!(
        matches!(
            step,
            ErrandStep::NoCertificate {
                reason: NoCertificate::NotOne,
                owned: 0,
                answered: None,
            }
        ),
        "la ventana lo enseña con su motivo: {step:?}"
    );
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "a la sede no se le ha dicho nada todavia"
    );
    assert!(
        live.current().is_some(),
        "y el tramite sigue vivo: instalar uno todavia lo arregla"
    );
    assert_eq!(
        live.the_request().as_ref(),
        Some(&url),
        "con la peticion apuntada, volver a mirar no reinicia nada"
    );
}
#[test]
#[expect(clippy::too_many_lines)]
fn on_the_signing_path_an_empty_keystore_stops_before_anything_is_written() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");

    let live = a_live();
    assert!(
        live.begin(Errand::of(
            NegotiatedCredential::Required(a_credential()),
            ArrivalMode::Awaited,
            a_codec()
        )),
        "la plaza es suya"
    );
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);

    let step = consent_to_sign(
        &a_desk(
            &engine,
            &policies,
            &[],
            home.path(),
            &listed,
            &opened,
            &memory,
            &scratch,
        ),
        &signature_requested(&a_signature("sign", "")),
        Vec::new(),
        &live,
    );

    assert!(
        matches!(
            step,
            ErrandStep::NoCertificate {
                reason: NoCertificate::NotOne,
                owned: 0,
                answered: None,
            }
        ),
        "sin ni un certificado la ventana lo enseña con su motivo: {step:?}"
    );
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "a la sede no se le ha dicho nada todavia"
    );
    assert!(
        live.current().is_some(),
        "y el tramite sigue vivo: instalar uno todavia lo arregla"
    );
    assert!(
        !scratch.exists(),
        "y no se ha escrito el fichero de paso: la decision llega antes"
    );

    let inadmissible = a_live();
    assert!(
        inadmissible.begin(Errand::of(
            NegotiatedCredential::Required(a_credential()),
            ArrivalMode::Awaited,
            a_codec()
        )),
        "la plaza es suya"
    );
    let step = consent_to_sign(
        &a_desk(
            &engine,
            &policies,
            &[],
            home.path(),
            &listed,
            &opened,
            &memory,
            &scratch,
        ),
        &signature_requested(&a_signature_over(b"esto no es un PDF", "sign", "")),
        Vec::new(),
        &inadmissible,
    );
    let ErrandStep::Answering(reply) = step else {
        panic!("la admisibilidad va primero, y eso no es un PDF: {step:?}");
    };
    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(SafCode::InvalidPdf).on_the_wire(),
        "el almacen vacio no le roba el turno a la admisibilidad"
    );
}
#[test]
fn leaving_the_no_certificate_screen_cancels_the_errand() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let engine = AnEngine::answering(&[]);

    let live = a_live();
    assert!(
        live.begin(Errand::of(
            NegotiatedCredential::Required(a_credential()),
            ArrivalMode::Awaited,
            a_codec()
        )),
        "la plaza es suya"
    );
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let url = arriving_over_the_channel(&format!(
        "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"
    ));
    live.keep_the_request(url.clone());
    consent_for(
        &engine,
        &requested(&url),
        Vec::new(),
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
        &live,
    );

    declined(&live);

    assert_eq!(
        what_the_site_received(&mut wire).as_deref(),
        Some(crate::site::domain::protocol::CANCELLED),
        "la sede recibe su CANCEL"
    );
    assert!(
        live.the_request().is_none(),
        "y no queda nada que reatender"
    );
}

#[test]
fn the_two_parameters_outside_properties_do_not_skip_the_consent() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    memory
        .remember_configuration(
            &crate::signing::application::configuration_memory::Configuration {
                honour_automatic_selection: true,
                ..Default::default()
            },
        )
        .expect("la memoria de pruebas escribe");
    let ours = vec![a_usable_certificate("EL UNICO")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let url = an_operation("&headless=true&mandatoryCertSelection=false");

    let step = consent_for(
        &AnEngine::answering(&[&[0]]),
        &requested(&url),
        ours,
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
        &live,
    );

    let ErrandStep::AskingForConsent {
        certificates: rows, ..
    } = step
    else {
        panic!("solo cuentan dentro de 'properties': {step:?}");
    };
    assert_eq!(rows.len(), 1, "uno solo se consiente igual");
}
#[test]
fn the_two_parameters_of_the_automatic_selection_are_read_only_by_the_protocol() {
    let production = concat!(
        include_str!("../mod.rs"),
        include_str!("../desk.rs"),
        include_str!("../replies.rs"),
        include_str!("../state.rs")
    );

    for parameter in ["\"headless\"", "\"mandatoryCertSelection\""] {
        assert!(
            !production.contains(parameter),
            "{parameter} se lee fuera de 'properties': la regla de la seleccion automatica se partiria"
        );
    }
}
#[test]
fn a_site_that_excludes_them_all_gets_the_code_of_an_empty_keystore() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let url = an_operation("");

    let step = consent_for(
        &AnEngine::answering(&[&[]]),
        &requested(&url),
        ours,
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
        &live,
    );

    let ErrandStep::NoCertificate {
        reason,
        owned,
        answered: Some(reply),
    } = step
    else {
        panic!("no hay nada que consentir: {step:?}");
    };
    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(SafCode::NoCertificatesInKeystore).on_the_wire()
    );
    assert!(
        reply.refusal().is_some(),
        "la ventana enseña la situacion entera"
    );
    assert_eq!(reason, NoCertificate::TheSiteExcludedThemAll);
    assert_eq!(owned, 1, "y cuantos tiene la persona, que es su almacen");
}
