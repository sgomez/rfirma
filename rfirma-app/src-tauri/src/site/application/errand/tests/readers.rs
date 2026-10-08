//! Pruebas del trámite de sede cuando cambian las tarjetas: la lista del consentimiento con el filtro de la sede.

use super::support::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::application::tests::a_memory;
use crate::site::application::errand::*;

fn labels(certificates: &[TokenCertificate]) -> Vec<&str> {
    certificates
        .iter()
        .map(|certificate| certificate.reference().label())
        .collect()
}

#[test]
fn a_card_arriving_during_the_consent_is_screened_by_the_filter_of_the_site() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[&[0], &[1]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let mut neighbours = a_neighbourhood(home.path(), &listed, &opened, &memory);
    neighbours.ours = ours.clone();
    let desk = a_desk(&engine, &policies, &neighbours, &scratch);
    let live = a_live();
    let (handle, _wire) = the_wire();
    attend(&desk, an_operation(""), handle, &live).expect("hay codec negociado");
    let now = vec![
        ours[0].clone(),
        a_usable_certificate("AUTENTICACION"),
        a_usable_certificate("OTRA"),
    ];

    let after = after_the_readers(&desk, now, &live);

    let AfterTheReaders::Accepted(accepted) = after else {
        panic!("el consentimiento enseña una lista: {after:?}");
    };
    assert_eq!(labels(&accepted), ["AUTENTICACION"]);
}

#[test]
fn with_no_certificate_the_errand_looks_again_when_a_card_arrives() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let rejected = a_usable_certificate("OTRA");
    let arrived = vec![rejected.clone(), a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&arrived);
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[&[], &[1]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let mut before = a_neighbourhood(home.path(), &listed, &opened, &memory);
    before.ours = vec![rejected];
    let live = a_live();
    let (handle, _wire) = the_wire();
    let first = attend(
        &a_desk(&engine, &policies, &before, &scratch),
        an_operation(""),
        handle,
        &live,
    );
    assert!(
        matches!(live.moment(), Some(Moment::NoCertificate { .. })),
        "sin la tarjeta no hay certificado que la sede acepte: {first:?}"
    );
    let mut with_the_card = a_neighbourhood(home.path(), &listed, &opened, &memory);
    with_the_card.ours = arrived.clone();

    let after = after_the_readers(
        &a_desk(&engine, &policies, &with_the_card, &scratch),
        arrived,
        &live,
    );

    assert!(
        matches!(
            after,
            AfterTheReaders::LookedAgain(Some(ErrandStep::AskingForConsent { .. }))
        ),
        "{after:?}"
    );
}

#[test]
fn a_moment_that_shows_no_list_is_left_alone() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let neighbours = a_neighbourhood(home.path(), &listed, &opened, &memory);

    let after = after_the_readers(
        &a_desk(&engine, &policies, &neighbours, &scratch),
        ours,
        &a_live(),
    );

    assert!(matches!(after, AfterTheReaders::Unchanged), "{after:?}");
}
