//! Pruebas de checkSignatures: la validación de las firmas previas antes de firmar.

use std::path::Path;

use super::support::*;
use super::support_requests::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::certificates::ListedCertificates;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::adapters::memory::Memory;
use crate::signing::application::tests::a_memory;
use crate::signing::domain::bridge::SignatureVerdict;
use crate::site::adapters::frontier;
use crate::site::application::errand::*;
use crate::site::application::tests::AValidator;
use crate::site::domain::protocol::{SafCode, WireAnswer};
use base64::Engine as _;

/// El bloque de `properties` que pide la sede, en Base64 URL-safe.
fn properties_of(block: &str) -> String {
    base64::engine::general_purpose::URL_SAFE.encode(block)
}

/// Una mesa que lista los certificados dados y valida las firmas previas con el doble que se le diga.
#[expect(
    clippy::too_many_arguments,
    reason = "es el constructor de un tipo de nueve campos, no una interfaz"
)]
fn a_desk_that_validates<'a>(
    engine: &'a AnEngine,
    policies: &'a APolicyEngine,
    validation: &'a AValidator,
    home: &'a Path,
    listed: &'a ListedCertificates,
    opened: &'a OpenedDocuments,
    memory: &'a Memory,
    scratch: &'a Path,
    ours: &[TokenCertificate],
) -> ErrandDesk<'a, AnEngine, APolicyEngine, TheNeighbours<'a>> {
    let mut desk = a_desk(engine, policies, &[], home, listed, opened, memory, scratch);
    desk.validation = validation;
    desk.neighbours.ours = ours.to_vec();
    desk
}

/// El consentimiento de una firma cuya sede pidió `checkSignatures`, con el veredicto que se le diga.
fn a_checked_consent(verdict: SignatureVerdict, pdf: &[u8], properties: &str) -> ErrandStep {
    a_checked_consent_to("sign", verdict, pdf, properties)
}

/// Lo mismo, para la operación que se le diga.
fn a_checked_consent_to(
    verb: &str,
    verdict: SignatureVerdict,
    pdf: &[u8],
    properties: &str,
) -> ErrandStep {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let (handle, _wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("checkSignatures=true\n");
    let validation = AValidator::saying(verdict);
    let scratch = home.path().join("errand");
    let desk = a_desk_that_validates(
        &engine,
        &policies,
        &validation,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
        &ours,
    );

    let step = consent_to_sign(
        &desk,
        &signature_requested(&a_signature_over(pdf, verb, properties)),
        ours.clone(),
        &live,
    );

    assert_eq!(
        validation.asked(),
        vec![base64::engine::general_purpose::STANDARD.encode(pdf)],
        "al validador se le da el documento entero en Base64, una sola vez"
    );
    step
}

#[test]
fn a_first_signature_over_a_document_without_signatures_passes_the_check() {
    let asked = a_checked_consent(SignatureVerdict::Unsigned, A_PDF, "");

    let ErrandStep::AskingToSign(consent) = asked else {
        panic!("una primera firma no necesita firmas previas: {asked:?}");
    };
    assert!(
        !consent.from_the_site.contains_key("checkSignatures"),
        "la clave la interpreta el tramite y no cruza al puente"
    );
}

#[test]
fn a_cosignature_over_a_document_without_signatures_is_answered_with_the_code_of_an_invalid_signature(
) {
    let asked = a_checked_consent_to("cosign", SignatureVerdict::Unsigned, A_PDF, "");

    let ErrandStep::Answering(SiteOutcome::Refused(refusal)) = &asked else {
        panic!("una multifirma sin firmas que validar se rechaza: {asked:?}");
    };
    assert_eq!(frontier::told(refusal).1, SafCode::InvalidSignature);
}

#[test]
fn a_previous_signature_that_does_not_hold_is_answered_with_the_code_of_an_invalid_signature() {
    let asked = a_checked_consent(
        SignatureVerdict::Invalid {
            reason: "la huella no cuadra".to_owned(),
        },
        A_PDF_SIGNED_BY_SOMETHING_ELSE,
        "",
    );

    let ErrandStep::Answering(SiteOutcome::Refused(refusal)) = &asked else {
        panic!("la sede recibe un rechazo: {asked:?}");
    };
    let (failure, code) = frontier::told(refusal);
    assert_eq!(code, SafCode::InvalidSignature);
    assert_eq!(failure.situation, "invalidSignature");
    assert_eq!(
        on_the_wire(match &asked {
            ErrandStep::Answering(outcome) => outcome,
            other => panic!("{other:?}"),
        }),
        WireAnswer::refused(SafCode::InvalidSignature).on_the_wire(),
        "lo que sale es lo que construye WireAnswer, no un codigo acunado"
    );
}

#[test]
fn a_confirmation_needed_by_a_headless_site_is_answered_with_the_code_of_confirmation_needed() {
    let asked = a_checked_consent(
        SignatureVerdict::ConfirmationNeeded {
            parameter: "allowSigningLtsSignature".to_owned(),
            message_code: "ProtocolLauncher.65".to_owned(),
        },
        A_PDF_SIGNED_BY_SOMETHING_ELSE,
        &format!("&properties={}", properties_of("headless=true\n")),
    );

    let ErrandStep::Answering(SiteOutcome::Refused(refusal)) = &asked else {
        panic!("sin nadie a quien preguntar la sede recibe un rechazo: {asked:?}");
    };
    let (failure, code) = frontier::told(refusal);
    assert_eq!(code, SafCode::ConfirmationNeeded);
    assert_eq!(failure.situation, "confirmationNeeded");
    assert_eq!(failure.detail, "ProtocolLauncher.65");
}

#[test]
fn a_confirmation_needed_without_headless_opens_the_confirming_moment() {
    let asked = a_checked_consent(
        SignatureVerdict::ConfirmationNeeded {
            parameter: "allowSigningLtsSignature".to_owned(),
            message_code: "ProtocolLauncher.65".to_owned(),
        },
        A_PDF_SIGNED_BY_SOMETHING_ELSE,
        "",
    );

    let ErrandStep::AskingToConfirm(consent) = &asked else {
        panic!("el tramite pregunta antes de seguir: {asked:?}");
    };
    assert_eq!(consent.parameter, "allowSigningLtsSignature");
    assert_eq!(consent.message_code, "ProtocolLauncher.65");
    assert_eq!(
        asked.moment(),
        Some(Moment::AskingToConfirm {
            message_code: "ProtocolLauncher.65".to_owned()
        }),
        "el momento nuevo cruza a la ventana"
    );
}

#[test]
fn a_validator_that_breaks_is_answered_with_the_code_of_the_bridge() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let (handle, _wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("checkSignatures=true\n");
    let validation = AValidator::that_breaks();
    let scratch = home.path().join("errand");
    let desk = a_desk_that_validates(
        &engine,
        &policies,
        &validation,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
        &ours,
    );

    let step = consent_to_sign(
        &desk,
        &signature_requested(&a_signature_over(A_PDF, "sign", "")),
        ours,
        &live,
    );

    let ErrandStep::Answering(SiteOutcome::Refused(refusal)) = &step else {
        panic!("la sede recibe un rechazo: {step:?}");
    };
    assert_eq!(frontier::code_of(refusal), SafCode::SignatureFailed);
}

#[test]
fn going_on_from_the_confirmation_fixes_the_key_and_checks_the_signatures_again() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("checkSignatures=true\n");
    let validation = AValidator::saying(SignatureVerdict::ConfirmationNeeded {
        parameter: "allowSigningLtsSignature".to_owned(),
        message_code: "ProtocolLauncher.65".to_owned(),
    });
    let scratch = home.path().join("errand");
    let desk = a_desk_that_validates(
        &engine,
        &policies,
        &validation,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
        &ours,
    );

    let url = a_signature_over(A_PDF_SIGNED_BY_SOMETHING_ELSE, "sign", "");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let ErrandStep::AskingToConfirm(_) = remembered(&live, step) else {
        panic!("el tramite pregunta antes de seguir");
    };

    let again = confirm(&desk, &live).expect("hay una confirmacion pendiente");

    assert_eq!(
        validation.asked().len(),
        2,
        "seguir repite la validacion con la clave ya fijada"
    );
    let ErrandStep::AskingToConfirm(consent) = &again else {
        panic!("el validador sigue pidiendo confirmacion: {again:?}");
    };
    assert_eq!(
        consent.confirmed.get("allowSigningLtsSignature").cloned(),
        Some("true".to_owned()),
        "la clave confirmada se conserva para la siguiente vuelta"
    );
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "confirmar no escribe nada en el cable"
    );
}

#[test]
fn a_confirmation_that_is_declined_answers_the_site_with_a_cancel() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("checkSignatures=true\n");
    let validation = AValidator::saying(SignatureVerdict::ConfirmationNeeded {
        parameter: "allowSigningLtsSignature".to_owned(),
        message_code: "ProtocolLauncher.65".to_owned(),
    });
    let scratch = home.path().join("errand");
    let desk = a_desk_that_validates(
        &engine,
        &policies,
        &validation,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
        &ours,
    );

    let url = a_signature_over(A_PDF_SIGNED_BY_SOMETHING_ELSE, "sign", "");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    assert!(matches!(
        remembered(&live, step),
        ErrandStep::AskingToConfirm(_)
    ));

    let reply = decline(&live);

    assert!(matches!(reply, SiteOutcome::Cancelled), "{reply:?}");
    assert_eq!(
        what_the_site_received(&mut wire),
        Some("CANCEL".to_owned()),
        "cancelar la confirmacion contesta CANCEL"
    );
}

#[test]
fn a_site_that_does_not_ask_to_check_signatures_never_asks_the_validator() {
    let asked = a_consent_to_sign_over(A_PDF_SIGNED_BY_SOMETHING_ELSE, "");

    assert!(
        matches!(asked, ErrandStep::AskingToSign(_)),
        "sin checkSignatures nada cambia: {asked:?}"
    );
}

#[test]
fn a_local_batch_ignores_check_signatures_as_the_original_does() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("checkSignatures=true\n");
    let scratch = home.path().join("errand");
    let desk = a_desk_for_the_local_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
        &ours,
    );

    let url = a_local_batch(&format!(
        "&properties={}",
        properties_of("checkSignatures=true\n")
    ));
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let ErrandStep::AskingToSignTheLocalBatch(asked) = remembered(&live, step) else {
        panic!("un lote local pide consentimiento");
    };

    consent(&desk, &asked.certificates[0].id, &live).expect("el certificado sirve");
    finish_the_local_batch(&desk, &the_typed_secret(), &live).expect("el lote local contesta");

    let result = the_batch_result(&mut wire);
    assert_eq!(
        result.matches("\"result\":\"DONE_AND_SAVED\"").count(),
        3,
        "el lote firma sus tres elementos sin preguntar por las firmas previas"
    );
}
