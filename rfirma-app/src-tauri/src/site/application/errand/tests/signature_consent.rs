//! Pruebas del consentimiento de firma: politica, firmas sin registrar y sign_and_save sin dat.

use super::support::*;
use super::support_requests::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::certificates::ListedCertificates;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::signing::application::tests::a_memory;
use crate::signing::domain::bridge::{Format, XadesVariant};
use crate::site::application::errand::*;
use crate::site::domain::protocol::{ChannelMessage, SafCode, SignatureRound, WireAnswer};
use base64::Engine as _;

#[test]
fn a_pdf_with_signatures_it_cannot_read_is_asked_about_inside_the_consent() {
    let asked = a_consent_to_sign_over(A_PDF_SIGNED_BY_SOMETHING_ELSE, "");

    let ErrandStep::AskingToSign(consent) = asked else {
        panic!("no es un rechazo, es un aviso: {asked:?}");
    };
    assert!(consent.unregistered_signatures);
}
#[test]
fn an_ordinary_pdf_asks_about_no_unregistered_signature() {
    let asked = a_consent_to_sign("");

    let ErrandStep::AskingToSign(consent) = asked else {
        panic!("hay certificado que la sede acepta: {asked:?}");
    };
    assert!(!consent.unregistered_signatures);
}
#[test]
fn a_site_that_allows_unregistered_signatures_does_not_skip_the_question() {
    let asked = a_consent_to_sign_over(
        A_PDF_SIGNED_BY_SOMETHING_ELSE,
        "allowCosigningUnregisteredSignatures=true\n",
    );

    let ErrandStep::AskingToSign(consent) = asked else {
        panic!("se pregunta igual: {asked:?}");
    };
    assert!(consent.unregistered_signatures);
    assert!(
        !consent
            .from_the_site
            .contains_key("allowCosigningUnregisteredSignatures"),
        "al puente solo se le manda tras el consentimiento"
    );
}
#[test]
fn a_site_that_forbids_unregistered_signatures_is_answered_with_a_cancel() {
    let asked = a_consent_to_sign_over(
        A_PDF_SIGNED_BY_SOMETHING_ELSE,
        "allowCosigningUnregisteredSignatures=false\n",
    );

    let ErrandStep::Answering(reply) = asked else {
        panic!("la sede ya contesto que no: {asked:?}");
    };
    assert_eq!(on_the_wire(&reply), "CANCEL");
}
#[test]
fn a_site_that_forbids_unregistered_signatures_still_signs_an_ordinary_pdf() {
    let asked = a_consent_to_sign("allowCosigningUnregisteredSignatures=false\n");

    let ErrandStep::AskingToSign(consent) = asked else {
        panic!("no hay nada que rechazar: {asked:?}");
    };
    assert!(!consent.unregistered_signatures);
}
#[test]
fn a_policy_that_cannot_be_applied_is_answered_with_the_code_of_an_invalid_policy() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::that_refuses_the_policy();
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
        ours.clone(),
        &live,
    );

    let ErrandStep::Answering(reply) = step else {
        panic!("la politica no se puede aplicar: {step:?}");
    };
    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(SafCode::InvalidPolicy).on_the_wire()
    );
    assert!(
        !scratch.exists(),
        "no se ha escrito nada: la politica se mira antes que el documento"
    );
}
#[test]
fn a_document_that_is_not_a_pdf_is_refused_before_anything_is_written() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let text = format!(
        "afirma://sign?op=sign&idsession={CREDENTIAL}&format=PAdES&algorithm=SHA256&dat={}",
        base64::engine::general_purpose::URL_SAFE.encode(b"esto no es un PDF")
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };

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
        &signature_requested(&url),
        ours.clone(),
        &live,
    );

    let ErrandStep::Answering(reply) = step else {
        panic!("eso no es un PDF: {step:?}");
    };
    assert_eq!(
        on_the_wire(&reply),
        WireAnswer::refused(SafCode::InvalidPdf).on_the_wire()
    );
    assert!(!scratch.exists(), "no se ha escrito nada");
}
#[test]
fn a_countersignature_is_shown_with_the_code_of_an_unsupported_operation() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");

    let step = attend_operation(
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
        &a_signature("countersign", ""),
        decoded(&a_signature("countersign", "")),
        &live,
    );

    let ErrandStep::ShowingTheRefusal(refusal) = step else {
        panic!("countersign no existe en PAdES: {step:?}");
    };
    assert_eq!(
        refusal.answer().on_the_wire(),
        WireAnswer::refused(SafCode::UnsupportedOperation).on_the_wire()
    );
}

#[test]
fn signing_and_saving_without_dat_opens_the_loading_moment_with_the_sites_hints() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let live = a_live();
    let url = a_sign_and_save_without_dat("");

    let step = attend_operation(
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
        &url,
        decoded(&url),
        &live,
    );

    let ErrandStep::Loading(consent) = step else {
        panic!("sin 'dat' se abre el selector de un solo fichero: {step:?}");
    };
    assert!(!consent.multiple, "signandsave nunca pide varios ficheros");
    assert_eq!(consent.extensions, ["pdf"]);
    assert_eq!(consent.description.as_deref(), Some("PDF"));
    assert_eq!(consent.starting_folder.as_deref(), Some("/home/persona"));
    assert!(
        consent.to_sign.is_some(),
        "lo elegido continua el tramite, no vuelve a la sede"
    );
    assert!(!scratch.exists(), "y no ha escrito nada todavia");
}

#[test]
fn a_selector_declined_for_sign_and_save_without_dat_answers_cancel() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_without_any_store(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    let live = a_live();
    let (handle, mut wire) = the_wire();

    let step = attend(&desk, a_sign_and_save_without_dat(""), handle, &live).expect("hay codec");
    assert!(matches!(step, ErrandStep::Loading(_)));

    let outcome = crate::site::application::errand::decline(&live);

    assert!(matches!(outcome, SiteOutcome::Cancelled));
    assert_eq!(what_the_site_received(&mut wire).as_deref(), Some("CANCEL"));
}

#[test]
fn a_document_chosen_for_sign_and_save_that_disappears_is_answered_with_saf_25() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_without_any_store(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    let live = a_live();
    let (handle, mut wire) = the_wire();

    let step = attend(&desk, a_sign_and_save_without_dat(""), handle, &live).expect("hay codec");
    assert!(matches!(step, ErrandStep::Loading(_)));

    let (handle, mut wire2) = the_wire();
    live.answer_through(handle);
    let missing = home.path().join("no-existe.pdf");
    let moved = crate::site::application::errand::document_chosen(
        &desk,
        &[("no-existe.pdf".to_owned(), missing)],
        &live,
    );

    assert!(
        matches!(moved, LoadCompletion::Delivered(_)),
        "el rechazo ya contesta a la sede"
    );
    let _ = what_the_site_received(&mut wire);
    assert!(
        what_the_site_received(&mut wire2).is_some_and(|line| line.starts_with("SAF_25")),
        "sale el codigo del catalogo"
    );
}

#[test]
fn choosing_the_document_for_sign_and_save_reaches_asking_to_sign_with_the_saving_hints() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0]]);
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
    let request = sign_and_save_requested(&a_sign_and_save_without_dat(""));
    assert_eq!(
        request.document(),
        None,
        "es la sin 'dat' que se firma aqui"
    );

    let step = crate::site::application::errand::consent_to_sign_with_chosen_document(
        &desk,
        crate::site::application::errand::PendingSignature::SigningAndSaving(request),
        A_PDF.to_vec(),
        None,
        ours,
        &live,
    );

    let ErrandStep::AskingToSign(consent) = step else {
        panic!("con documento elegido y certificado aceptado se pide firma: {step:?}");
    };
    assert_eq!(consent.round, SignatureRound::First);
    let saving = consent.saving.expect("signandsave trae pistas de guardado");
    assert_eq!(saving.filename, "firma.pdf");
    assert_eq!(saving.extensions, ["pdf", "p7s"]);
    assert_eq!(saving.description.as_deref(), Some("Documentos"));
    assert_eq!(saving.starting_folder.as_deref(), Some("/home/persona"));
}

/// El contenedor ASiC-S de CAdES era el contraejemplo del formato sin puente, y
/// ya no lo es: el trámite lo lleva al consentimiento como a cualquier otro.
#[test]
fn the_asic_s_container_of_cades_reaches_the_consent_like_any_other_format() {
    let format = "CAdES-ASiC-S";
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
        &signature_requested(&a_signature_asking_for(format, A_PDF)),
        ours.clone(),
        &live,
    );

    let ErrandStep::AskingToSign(consent) = step else {
        panic!("el puente ya atiende '{format}': {step:?}");
    };
    assert_eq!(consent.round, SignatureRound::First);
}

#[test]
fn explicit_mode_with_xades_is_refused_before_asking_for_consent() {
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
        &signature_requested(&an_explicit_mode_signature(
            "XAdES",
            b"<?xml version=\"1.0\"?><documento/>",
        )),
        ours.clone(),
        &live,
    );

    let ErrandStep::ShowingTheRefusal(refusal) = step else {
        panic!("la XAdES explicita firma la huella SHA-1: {step:?}");
    };
    assert_eq!(refusal.code(), SafCode::UnsupportedFormat);
}

/// Y los formatos que el puente sí atiende siguen su curso hasta el consentimiento.
#[test]
fn the_format_the_bridge_attends_goes_on_to_the_consent_as_it_did() {
    for (format, document, expected) in [
        ("PAdES", A_PDF, Format::Pades),
        ("auto", &[0x00, 0x01, 0x02][..], Format::Cades),
        (
            "auto",
            b"<?xml version=\"1.0\"?><documento/>".as_slice(),
            Format::Xades(XadesVariant::Enveloping),
        ),
    ] {
        let home = tempfile::tempdir().expect("debería haber directorio temporal");
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
            &signature_requested(&a_signature_asking_for(format, document)),
            ours.clone(),
            &live,
        );

        let ErrandStep::AskingToSign(consent) = step else {
            panic!("'{format}' se firma igual que antes: {step:?}");
        };
        assert_eq!(consent.format, expected, "format={format}");
    }
}

#[test]
fn the_moment_of_a_consent_carries_the_format_the_site_asked_for() {
    let step = a_consent_to_sign("");

    let Some(Moment::AskingToSign { format, .. }) = step.moment() else {
        panic!("hay un certificado que la sede acepta: {step:?}");
    };
    assert_eq!(format, Format::Pades);
}
