//! Pruebas de la selección automática que pide la sede con `headless` o `mandatoryCertSelection=false`.

use super::support::*;
use super::support_requests::A_PDF_SIGNED_BY_SOMETHING_ELSE;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::application::tests::{a_memory, A_CADES_SIGNATURE};
use crate::site::application::errand::*;
use crate::site::domain::protocol::{AfirmaUrl, ChannelMessage};
use base64::Engine as _;

const HEADLESS: &str = "headless=true\n";
const NOT_MANDATORY: &str = "mandatoryCertSelection=false\n";
const MANDATORY: &str = "mandatoryCertSelection=true\n";

const A_JSON_BATCH: &str = "{\"algorithm\":\"SHA256\",\"stoponerror\":false,\"singlesigns\":[{\"id\":\"001\",\"datareference\":\"AAAA\"}]}";

/// Lo que el trámite hace con la operación: contestar sin ventana, o pedir el consentimiento.
#[derive(Debug, PartialEq, Eq)]
enum Consent {
    Skipped,
    Asked,
}

/// Una operación del canal con el `properties` dado y lo que se le añada detrás.
fn an_operation_declaring(query: &str, declared: &str, document: &[u8]) -> AfirmaUrl {
    let engine = base64::engine::general_purpose::URL_SAFE;
    let text = format!(
        "afirma://x?{query}&idsession={CREDENTIAL}&dat={}&properties={}",
        engine.encode(document),
        engine.encode(declared)
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

fn a_selection(declared: &str) -> AfirmaUrl {
    an_operation_declaring("op=selectcert", declared, b"")
}

fn a_signature(verb: &str, declared: &str) -> AfirmaUrl {
    an_operation_declaring(
        &format!("op={verb}&format=PAdES&algorithm=SHA256withRSA"),
        declared,
        b"%PDF-1.7\n",
    )
}

fn a_countersignature(declared: &str) -> AfirmaUrl {
    an_operation_declaring(
        "op=countersign&format=CAdES&algorithm=SHA256withRSA",
        declared,
        A_CADES_SIGNATURE,
    )
}

fn a_sign_and_save(declared: &str) -> AfirmaUrl {
    an_operation_declaring(
        "op=signandsave&cop=sign&format=PAdES&algorithm=SHA256withRSA&filename=firma.pdf",
        declared,
        b"%PDF-1.7\n",
    )
}

fn a_remote_batch(declared: &str) -> AfirmaUrl {
    an_operation_declaring(
        "op=batch&jsonbatch=true&batchpresignerurl=https%3A%2F%2Fpresigner.example%2Fpre&\
         batchpostsignerurl=https%3A%2F%2Fpostsigner.example%2Fpost",
        declared,
        A_JSON_BATCH.as_bytes(),
    )
}

fn a_local_batch(declared: &str) -> AfirmaUrl {
    an_operation_declaring(
        "op=batch&jsonbatch=true&localBatchProcess=true",
        declared,
        A_JSON_BATCH.as_bytes(),
    )
}

/// Atiende la operación con los certificados dados, todos aceptados por la sede, y la preferencia dada.
fn attended(url: &AfirmaUrl, ours: Vec<TokenCertificate>, honoured: bool) -> ErrandStep {
    attended_by(url, ours, honoured, &a_live())
}

fn attended_by(
    url: &AfirmaUrl,
    ours: Vec<TokenCertificate>,
    honoured: bool,
    live: &LiveErrand,
) -> ErrandStep {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    memory
        .remember_configuration(&Configuration {
            honour_automatic_selection: honoured,
            ..Configuration::default()
        })
        .expect("la memoria de pruebas escribe");
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let everyone: Vec<usize> = (0..ours.len()).collect();
    let engine = AnEngine::answering(&[&everyone]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let mut desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    desk.neighbours.ours = ours;
    attend_operation(&desk, url, decoded(url), live)
}

fn consent_of(step: &ErrandStep) -> Consent {
    match step {
        ErrandStep::Answering(SiteOutcome::Certificate(_)) => Consent::Skipped,
        ErrandStep::AskingForConsent { .. } => Consent::Asked,
        ErrandStep::AskingToSign(consent) if consent.without_asking => Consent::Skipped,
        ErrandStep::AskingToSignTheBatch(consent) if consent.without_asking => Consent::Skipped,
        ErrandStep::AskingToSignTheLocalBatch(consent) if consent.without_asking => {
            Consent::Skipped
        }
        ErrandStep::AskingToSign(_)
        | ErrandStep::AskingToSignTheBatch(_)
        | ErrandStep::AskingToSignTheLocalBatch(_) => Consent::Asked,
        other => panic!("la operacion llega al consentimiento: {other:?}"),
    }
}

fn one() -> Vec<TokenCertificate> {
    vec![a_usable_certificate("EL UNICO")]
}

fn two() -> Vec<TokenCertificate> {
    vec![a_usable_certificate("FIRMA"), a_usable_certificate("OTRA")]
}

fn every_operation(declared: &str) -> [AfirmaUrl; 7] {
    [
        a_selection(declared),
        a_signature("sign", declared),
        a_signature("cosign", declared),
        a_countersignature(declared),
        a_sign_and_save(declared),
        a_remote_batch(declared),
        a_local_batch(declared),
    ]
}

#[test]
fn with_the_preference_one_candidate_and_either_parameter_skip_the_consent_of_every_operation() {
    for declared in [HEADLESS, NOT_MANDATORY] {
        for url in every_operation(declared) {
            let step = attended(&url, one(), true);

            assert_eq!(consent_of(&step), Consent::Skipped, "{declared:?} {url:?}");
        }
    }
}

#[test]
fn with_the_preference_two_candidates_are_still_asked_in_every_operation() {
    for declared in [HEADLESS, NOT_MANDATORY] {
        for url in every_operation(declared) {
            let step = attended(&url, two(), true);

            assert_eq!(consent_of(&step), Consent::Asked, "{declared:?} {url:?}");
        }
    }
}

#[test]
fn without_the_preference_the_consent_is_always_asked() {
    for declared in [HEADLESS, NOT_MANDATORY] {
        for url in every_operation(declared) {
            let step = attended(&url, one(), false);

            assert_eq!(consent_of(&step), Consent::Asked, "{declared:?} {url:?}");
        }
    }
}

#[test]
fn with_the_preference_a_site_that_keeps_the_choice_mandatory_is_asked() {
    for declared in [MANDATORY, ""] {
        for url in every_operation(declared) {
            let step = attended(&url, one(), true);

            assert_eq!(consent_of(&step), Consent::Asked, "{declared:?} {url:?}");
        }
    }
}

#[test]
fn a_signature_that_skips_the_consent_goes_on_with_its_only_certificate() {
    let step = attended(&a_signature("sign", NOT_MANDATORY), one(), true);

    let ErrandStep::AskingToSign(consent) = step else {
        panic!("hay algo que firmar: {step:?}");
    };
    assert_eq!(
        consent.already_chosen,
        consent.certificates.first().map(|row| row.id.clone())
    );
}

#[test]
fn a_certificate_stuck_among_two_candidates_is_preselected_but_still_asked() {
    let ours = two();
    let live = a_live();
    live.stick(ours[1].reference());
    let url = an_operation_declaring(
        "op=sign&format=PAdES&algorithm=SHA256withRSA&sticky=true",
        HEADLESS,
        b"%PDF-1.7\n",
    );

    let step = attended_by(&url, ours, true, &live);

    let ErrandStep::AskingToSign(consent) = step else {
        panic!("hay algo que firmar: {step:?}");
    };
    assert!(
        !consent.without_asking,
        "el fijado preselecciona, no contesta"
    );
    assert_eq!(
        consent.already_chosen.as_deref(),
        Some(consent.certificates[1].id.as_str())
    );
}

fn a_signature_over(verb: &str, declared: &str, document: &[u8]) -> AfirmaUrl {
    an_operation_declaring(
        &format!("op={verb}&format=PAdES&algorithm=SHA256withRSA"),
        declared,
        document,
    )
}

#[test]
fn with_the_preference_a_pdf_with_unregistered_signatures_is_still_asked() {
    for verb in ["sign", "cosign"] {
        let url = a_signature_over(verb, NOT_MANDATORY, A_PDF_SIGNED_BY_SOMETHING_ELSE);

        assert_eq!(
            consent_of(&attended(&url, one(), true)),
            Consent::Asked,
            "{verb}"
        );
    }
}

#[test]
fn with_the_preference_unregistered_signatures_the_site_allows_are_still_asked() {
    let declared = format!("{NOT_MANDATORY}allowCosigningUnregisteredSignatures=true\n");
    let url = a_signature_over("cosign", &declared, A_PDF_SIGNED_BY_SOMETHING_ELSE);

    assert_eq!(consent_of(&attended(&url, one(), true)), Consent::Asked);
}

#[test]
fn with_the_preference_a_sign_and_save_over_unregistered_signatures_is_still_asked() {
    let url = an_operation_declaring(
        "op=signandsave&cop=sign&format=PAdES&algorithm=SHA256withRSA&filename=firma.pdf",
        NOT_MANDATORY,
        A_PDF_SIGNED_BY_SOMETHING_ELSE,
    );

    assert_eq!(consent_of(&attended(&url, one(), true)), Consent::Asked);
}

#[test]
fn a_selection_answered_without_asking_sticks_its_certificate_when_the_site_asks() {
    let ours = one();
    let expected = ours[0].reference().clone();
    let live = a_live();
    let url = an_operation_declaring("op=selectcert&sticky=true", HEADLESS, b"");

    let step = attended_by(&url, ours, true, &live);

    assert_eq!(consent_of(&step), Consent::Skipped);
    assert_eq!(live.the_stuck(), Some(expected));
}
