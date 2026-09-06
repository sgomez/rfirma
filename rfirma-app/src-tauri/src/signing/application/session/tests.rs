use super::{
    admitted_bytes, begin, cancel, config_for, finish, is_live, sign_on_token, signed_document,
    signed_folder, take_signed_cycle, SigningSession,
};
use crate::commands::Failure;
use crate::documents::adapters::portal::PortalDocument;
use crate::documents::application::opened::OpenedDocuments;
use crate::fixtures::{a_certificate, a_memory, an_order, NoIsolate, NoToken};
use crate::identity::application::listed::ListedCertificates;
use crate::signing::adapters::orders::{PlacementOrder, SigningOrder};
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::domain::PageSet;

const SOURCE: &str = include_str!("../session.rs");

fn production_half() -> &'static str {
    half_of(SOURCE)
}

fn half_of(source: &'static str) -> &'static str {
    source
}

#[test]
fn the_pin_is_never_kept_in_the_half_open_cycle() {
    let session = production_half()
        .split_once("struct InFlight {")
        .expect("la sesión sigue aquí")
        .1
        .split_once("\n}")
        .expect("y tiene cuerpo")
        .0;

    assert!(
        !session.contains("pin"),
        "el PIN se está guardando: {session}"
    );
}

#[test]
fn the_seal_travels_apart_from_the_cycle_that_issued_it() {
    // ADR-0016.
    let session = production_half()
        .split_once("struct InFlight {")
        .expect("la sesión sigue aquí")
        .1;

    assert!(session.contains("seal: SessionSeal"));
}

#[test]
fn a_document_that_is_not_remembered_gets_no_row_when_it_is_signed() {
    let postsign = production_half()
        .split_once("pub fn finish(")
        .expect("la postfirma sigue aqui")
        .1;
    let before_the_row = postsign
        .split_once("recents::note_signed(")
        .expect("la postfirma anota la fila")
        .0;

    assert!(
        before_the_row.contains("if document.is_remembered() {"),
        "la fila del firmado se escribe sin preguntar si el documento se recuerda"
    );
}

#[test]
fn only_the_postsign_remembers_the_certificate() {
    let source = production_half();

    assert_eq!(
        source
            .matches("certificates::remember_the_certificate(")
            .count(),
        1,
        "se recuerda desde un solo sitio"
    );
    let postsign = source
        .split_once("pub fn finish(")
        .expect("la postfirma sigue aqui")
        .1;
    assert!(
        postsign.contains("certificates::remember_the_certificate("),
        "y ese sitio es la postfirma"
    );
}

#[test]
fn the_geometry_of_the_order_becomes_pades_points() {
    let certificate = a_certificate("FIRMA", &[]);

    let config = config_for(&an_order(), &certificate).expect("el recuadro cabe");

    let placement = config.placement.expect("la ventana coloco el recuadro");
    assert_eq!(placement.pages, PageSet::only_page(1));
    assert_eq!(placement.rect.lower_left_x, 72);
    assert_eq!(placement.rect.upper_right_y, 600);
}

#[test]
fn a_box_outside_the_page_is_refused_instead_of_being_clipped_in_silence() {
    let order = SigningOrder {
        placement: Some(PlacementOrder {
            rect: [72.0, 500.0, 900.0, 600.0],
            ..an_order().placement.expect("el andamio trae recuadro")
        }),
        ..an_order()
    };

    let failure = config_for(&order, &a_certificate("FIRMA", &[])).expect_err("se sale");

    assert_eq!(Failure::from(failure).situation, "boxOutOfPage");
}

#[test]
fn an_empty_reason_is_not_sent_at_all() {
    let config = config_for(&an_order(), &a_certificate("FIRMA", &[])).expect("cabe");

    assert_eq!(config.sign_reason, None);
}

#[test]
fn a_reason_that_was_written_does_travel() {
    let order = SigningOrder {
        reason: "Conforme".to_owned(),
        ..an_order()
    };

    let config = config_for(&order, &a_certificate("FIRMA", &[])).expect("cabe");

    assert_eq!(config.sign_reason.as_deref(), Some("Conforme"));
}

#[test]
fn there_is_nothing_to_finish_when_no_cycle_was_started() {
    let session = SigningSession::default();

    let Err(failure) = take_signed_cycle(&session) else {
        panic!("no hay ciclo abierto que llevarse");
    };

    assert_eq!(Failure::from(failure).situation, "unknown");
}

#[test]
fn there_is_nothing_to_open_before_the_first_signature_of_the_session() {
    let session = SigningSession::default();

    let Err(failure) = signed_document(&session) else {
        panic!("no se ha firmado nada todavia");
    };
    assert_eq!(Failure::from(failure).situation, "unknown");
    assert!(signed_folder(&session).is_err());
}

#[test]
fn the_two_openers_read_the_landing_the_postsign_left_behind() {
    let session = SigningSession::default();
    let folder = tempfile::tempdir().expect("deberia haber temporal");
    let landing = folder.path().join("contrato-firmado.pdf");
    *crate::lock(&session.delivered) = Some(landing.clone());

    assert_eq!(signed_document(&session).expect("hay firmado"), landing);
    assert_eq!(signed_folder(&session).expect("y carpeta"), folder.path());
}

#[test]
fn a_session_with_no_open_cycle_is_not_live() {
    assert!(!is_live(&SigningSession::default()));
}

#[test]
fn a_cancelled_session_is_not_live_either() {
    let session = SigningSession::default();

    cancel(&session);

    assert!(!is_live(&session));
}

#[test]
fn the_remembered_landing_never_leaves_the_backend() {
    let crossing = production_half()
        .split_once("pub struct SigningSession {")
        .expect("la sesion sigue aqui")
        .1
        .split_once("\n}")
        .expect("y tiene cuerpo")
        .0;

    assert!(
        crossing.contains("delivered"),
        "la sesion tiene que recordar donde cayo el firmado: {crossing}"
    );
    assert!(
        !crossing.contains("Serialize"),
        "la sesion no se serializa: si cruzara, cruzaria una ruta del anfitrion"
    );
}

#[test]
fn what_is_not_a_pdf_is_refused_before_the_pin() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let other = home.path().join("hoja.ods");
    std::fs::write(&other, b"PK\x03\x04").expect("deberia escribirse el temporal");

    let failure =
        admitted_bytes(&PortalDocument::opened(other)).expect_err("no es un PDF que firmar");

    assert_eq!(Failure::from(failure).situation, "notAPdf");
}

#[test]
fn a_document_that_is_gone_is_told_apart_from_one_that_is_not_a_pdf() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");

    let failure = admitted_bytes(&PortalDocument::opened(home.path().join("no-esta.pdf")))
        .expect_err("no esta");

    assert_eq!(Failure::from(failure).situation, "documentUnreadable");
}

#[test]
fn a_signature_cannot_begin_on_a_document_that_is_not_open() {
    let order = SigningOrder {
        document: "00000000000000000000000000000000".to_owned(),
        ..an_order()
    };

    let failure = begin(
        &order,
        &NoToken,
        &[],
        &ListedCertificates::new(),
        &OpenedDocuments::new(),
        &NoIsolate,
        &SigningSession::default(),
    )
    .expect_err("ese documento no esta abierto");

    assert_eq!(Failure::from(failure).situation, "documentUnreadable");
}

#[test]
fn the_postsign_stops_before_the_bridge_when_no_cycle_was_started() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");

    let failure = finish(
        &NoIsolate,
        &SigningSession::default(),
        &a_memory(home.path()),
        &Configuration::default(),
        home.path(),
    )
    .expect_err("no hay ciclo abierto");

    assert_eq!(Failure::from(failure).situation, "unknown");
}

#[test]
fn the_pin_has_nothing_to_sign_when_no_cycle_was_started() {
    let failure = sign_on_token(&NoToken, &SigningSession::default(), "1234")
        .expect_err("no hay ciclo abierto");

    assert_eq!(Failure::from(failure).situation, "unknown");
}

#[test]
fn cancelling_leaves_no_cycle_behind() {
    let session = SigningSession::default();

    cancel(&session);

    assert!(
        take_signed_cycle(&session).is_err(),
        "no queda ciclo que llevarse"
    );
}
