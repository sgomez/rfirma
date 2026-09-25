//! Pruebas del modo headless: la selección y la firma que no preguntan a la persona.

use super::support::*;
use super::support_requests::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::tests::{
    a_usable_certificate, an_expired_certificate, listed_from,
};
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::application::tests::a_memory;
use crate::site::application::errand::*;
use crate::site::domain::protocol::{SafCode, SiteVisibleSignature, WireAnswer};
use base64::Engine as _;

/// El `properties` con `headless=true`, ya en Base64 del protocolo.
fn headless_properties() -> String {
    base64::engine::general_purpose::URL_SAFE.encode(b"headless=true\n")
}

/// La selección de certificado del `headless` de la sede, con los certificados que se le digan y la
/// preferencia que lo respeta activada.
fn a_headless_selection(ours: Vec<TokenCertificate>, accepted: &[usize]) -> ErrandStep {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    memory
        .remember_configuration(&Configuration {
            honour_automatic_selection: true,
            ..Configuration::default()
        })
        .expect("la memoria de pruebas escribe");
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let engine = AnEngine::answering(&[accepted]);
    let url = arriving_over_the_channel(&format!(
        "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}&properties={}",
        headless_properties()
    ));

    consent_for(
        &engine,
        &requested(&url),
        ours,
        &a_neighbourhood(home.path(), &listed, opened_for_nobody(), &memory),
        &live,
    )
}

#[test]
fn headless_with_a_single_accepted_certificate_answers_without_asking() {
    let step = a_headless_selection(vec![a_usable_certificate("FIRMA")], &[0]);

    let ErrandStep::Answering(SiteOutcome::Certificate(_)) = step else {
        panic!("con un solo certificado admitido no se pregunta: {step:?}");
    };
}

#[test]
fn headless_with_a_single_expired_candidate_answers_saf19_without_asking() {
    let step = a_headless_selection(vec![an_expired_certificate("CADUCADO")], &[0]);

    let ErrandStep::NoCertificate {
        reason: NoCertificate::TheSiteExcludedThemAll,
        answered: Some(reply),
        ..
    } = step
    else {
        panic!("un unico candidato caducado no abre ventana en headless: {step:?}");
    };
    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(SafCode::NoCertificatesInKeystore).on_the_wire()
    );
}

#[test]
fn headless_with_an_expired_candidate_the_filter_admits_and_a_usable_one_still_asks() {
    let step = a_headless_selection(
        vec![
            an_expired_certificate("CADUCADO"),
            a_usable_certificate("FIRMA"),
        ],
        &[0, 1],
    );

    let ErrandStep::AskingForConsent { certificates, .. } = step else {
        panic!("el original cuenta el caducado que el filtro admite y pregunta: {step:?}");
    };
    assert_eq!(certificates.len(), 2);
}

#[test]
fn headless_with_two_accepted_certificates_still_asks() {
    let step = a_headless_selection(
        vec![
            a_usable_certificate("FIRMA"),
            a_usable_certificate("OTRA FIRMA"),
        ],
        &[0, 1],
    );

    let ErrandStep::AskingForConsent { certificates, .. } = step else {
        panic!("con dos certificados admitidos se pregunta: {step:?}");
    };
    assert_eq!(certificates.len(), 2);
}

#[test]
fn a_signature_under_headless_arrives_with_its_only_certificate_already_chosen() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");

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
        &signature_requested(&a_signature(
            "sign",
            &format!("&properties={}", headless_properties()),
        )),
        ours,
        &live,
    );

    let ErrandStep::AskingToSign(consent) = step else {
        panic!("hay algo que firmar: {step:?}");
    };
    assert_eq!(
        consent.already_chosen,
        consent.certificates.first().map(|row| row.id.clone()),
        "es la unica fila que la sede acepta"
    );
}

#[test]
fn a_signature_under_headless_with_a_single_expired_candidate_answers_saf19_without_asking() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![an_expired_certificate("CADUCADO")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");

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
        &signature_requested(&a_signature(
            "sign",
            &format!("&properties={}", headless_properties()),
        )),
        ours,
        &live,
    );

    let ErrandStep::NoCertificate {
        reason: NoCertificate::TheSiteExcludedThemAll,
        answered: Some(reply),
        ..
    } = step
    else {
        panic!("un unico candidato caducado no abre ventana en headless: {step:?}");
    };
    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(SafCode::NoCertificatesInKeystore).on_the_wire()
    );
}

#[test]
fn a_signature_without_headless_leaves_the_choice_open() {
    let asked = a_consent_to_sign("");

    let ErrandStep::AskingToSign(consent) = asked else {
        panic!("hay algo que firmar");
    };
    assert_eq!(consent.already_chosen, None);
}

#[test]
fn a_signature_without_headless_shows_an_expired_candidate_with_its_status_but_never_chooses_it() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![
        an_expired_certificate("CADUCADO"),
        a_usable_certificate("FIRMA"),
    ];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0, 1]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");

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
        ours,
        &live,
    );

    let ErrandStep::AskingToSign(consent) = step else {
        panic!("hay algo que firmar: {step:?}");
    };
    assert_eq!(
        consent.certificates.len(),
        2,
        "la ventana enseña el caducado, no lo oculta"
    );
    assert!(
        consent
            .certificates
            .iter()
            .any(|row| !row.status.is_usable()),
        "y con su estado, que dice que no se puede elegir"
    );
    assert_eq!(
        consent.already_chosen, None,
        "sin headless no se elige nadie por la persona"
    );
}

/// Sin `properties` legible no hay filtro, ni parámetros adicionales, ni recuadro que enseñar:
/// el consentimiento tiene que reflejar lo que de verdad se va a firmar.
#[test]
fn a_signature_with_unreadable_properties_is_consented_with_nothing_the_site_declared() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");

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
        &signature_requested(&a_signature("sign", "&properties=!!!!")),
        ours,
        &live,
    );

    let ErrandStep::AskingToSign(consent) = step else {
        panic!("un 'properties' ilegible no tumba la firma: {step:?}");
    };
    assert!(consent.filter.declares_nothing());
    assert!(consent.from_the_site.is_empty());
    assert!(matches!(consent.visible, SiteVisibleSignature::Declined));
}
