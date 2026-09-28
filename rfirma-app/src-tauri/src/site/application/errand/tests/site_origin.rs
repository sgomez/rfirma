//! Pruebas de cómo el origen de la operación sobrevive a su atención por el trámite.

use super::support::*;
use super::support_requests::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::signing::application::tests::a_memory;
use crate::site::application::errand::*;
use crate::site::domain::site_origin::SiteOrigin;

fn named() -> SiteOrigin {
    SiteOrigin::from_header(Some("https://sede.ejemplo.gob.es"))
}

#[test]
fn a_signature_consent_keeps_the_origin_the_operation_arrived_with() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let mut neighbours = a_neighbourhood(home.path(), &listed, &opened, &memory);
    neighbours.ours = ours;
    let desk = a_desk(&engine, &policies, &neighbours, &scratch);
    let (handle, _wire) = the_wire();

    live.note_origin(named());
    let step = attend(&desk, a_signature("sign", ""), handle, &live).expect("hay codec");

    assert!(matches!(step, ErrandStep::AskingToSign(_)));
    assert_eq!(
        live.origin(),
        named(),
        "el consentimiento de firma se atiende con el origen de la operacion que lo pidio"
    );
}

#[test]
fn the_lack_of_a_certificate_keeps_the_origin_the_operation_arrived_with() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let mut neighbours = a_neighbourhood(home.path(), &listed, &opened, &memory);
    neighbours.ours = ours;
    let desk = a_desk(&engine, &policies, &neighbours, &scratch);
    let (handle, _wire) = the_wire();

    live.note_origin(named());
    let step = attend(&desk, a_signature("sign", ""), handle, &live).expect("hay codec");

    assert!(matches!(step, ErrandStep::NoCertificate { .. }));
    assert_eq!(
        live.origin(),
        named(),
        "la falta de certificado no borra el origen de la operacion que la pidio"
    );
}

#[test]
fn a_second_operation_with_a_different_origin_replaces_the_first() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let mut neighbours = a_neighbourhood(home.path(), &listed, &opened, &memory);
    neighbours.ours = ours;
    let desk = a_desk(&engine, &policies, &neighbours, &scratch);

    live.note_origin(named());
    let (first, _first_wire) = the_wire();
    let _ = attend(&desk, a_signature("sign", ""), first, &live).expect("hay codec");
    assert_eq!(live.origin(), named());

    live.note_origin(SiteOrigin::absent());
    let (second, _second_wire) = the_wire();
    let _ = attend(&desk, a_signature("sign", ""), second, &live).expect("hay codec");

    assert_eq!(
        live.origin(),
        SiteOrigin::absent(),
        "la segunda operacion sustituye al origen de la primera, aunque no traiga uno"
    );
}

#[test]
fn before_any_operation_arrives_there_is_no_origin_to_attribute() {
    let live = a_live();

    assert_eq!(live.origin(), SiteOrigin::absent());
}
