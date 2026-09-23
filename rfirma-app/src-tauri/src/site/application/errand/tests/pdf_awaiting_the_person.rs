//! Pruebas del PDF que el original no firma sin preguntar: certificado, cifrado o con firmas no registradas.

use super::support::*;
use super::support_requests::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::signing::application::tests::a_memory;
use crate::site::adapters::frontier;
use crate::site::application::errand::*;
use crate::site::domain::protocol::SafCode;
use base64::Engine as _;

const A_CERTIFIED_PDF: &[u8] =
    b"%PDF-1.7\n9 0 obj\n<< /Type /Sig /Reference [ << /TransformMethod /DocMDP >> ] >>\nendobj\n";

const A_PASSWORD_PROTECTED_PDF: &[u8] =
    b"%PDF-1.7\ntrailer\n<< /Size 9 /Encrypt 8 0 R /Root 1 0 R >>\n";

const THE_PASSWORD: &str = "s3cr3t-of-the-pdf";

fn a_consent_over(pdf: &[u8], declared: &str) -> ErrandStep {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering(declared);
    let scratch = home.path().join("errand");
    let properties = base64::engine::general_purpose::URL_SAFE.encode(declared);

    consent_to_sign(
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
        &signature_requested(&a_signature_over(
            pdf,
            "sign",
            &format!("&properties={properties}"),
        )),
        ours,
        &live,
    )
}

fn the_code_of(step: &ErrandStep) -> SafCode {
    let ErrandStep::Answering(SiteOutcome::Refused(refusal)) = step else {
        panic!("la sede recibe un rechazo: {step:?}");
    };
    frontier::told(refusal).1
}

#[test]
fn a_certified_pdf_under_headless_is_answered_with_the_code_of_confirmation_needed() {
    let step = a_consent_over(A_CERTIFIED_PDF, "headless=true\n");

    assert_eq!(the_code_of(&step), SafCode::ConfirmationNeeded);
}

#[test]
fn a_password_protected_pdf_under_headless_is_answered_with_the_code_of_confirmation_needed() {
    let step = a_consent_over(A_PASSWORD_PROTECTED_PDF, "headless=true\n");

    assert_eq!(the_code_of(&step), SafCode::ConfirmationNeeded);
}

#[test]
fn a_pdf_with_unregistered_signatures_under_headless_is_answered_with_the_code_of_confirmation_needed(
) {
    let step = a_consent_over(A_PDF_SIGNED_BY_SOMETHING_ELSE, "headless=true\n");

    assert_eq!(the_code_of(&step), SafCode::ConfirmationNeeded);
}

#[test]
fn a_certified_pdf_without_headless_keeps_the_code_of_a_certified_pdf() {
    let step = a_consent_over(A_CERTIFIED_PDF, "");

    assert_eq!(the_code_of(&step), SafCode::PdfCertified);
}

#[test]
fn a_certified_pdf_the_site_forbids_keeps_the_code_of_a_certified_pdf_even_under_headless() {
    let step = a_consent_over(
        A_CERTIFIED_PDF,
        "headless=true\nallowSigningCertifiedPdfs=false\n",
    );

    assert_eq!(the_code_of(&step), SafCode::PdfCertified);
}

#[test]
fn a_password_protected_pdf_without_headless_keeps_the_code_of_a_wrong_password() {
    let step = a_consent_over(A_PASSWORD_PROTECTED_PDF, "");

    assert_eq!(the_code_of(&step), SafCode::PdfWrongPassword);
}

#[test]
fn allow_signing_certified_pdfs_lets_a_certified_pdf_through_under_headless() {
    let step = a_consent_over(
        A_CERTIFIED_PDF,
        "headless=true\nallowSigningCertifiedPdfs=true\n",
    );

    let ErrandStep::AskingToSign(consent) = step else {
        panic!("la sede ya lo permitio: {step:?}");
    };
    assert_eq!(
        consent
            .from_the_site
            .get("allowSigningCertifiedPdfs")
            .map(String::as_str),
        Some("true"),
        "el puente tiene que recibirlo para no volver a negarse"
    );
}

#[test]
fn the_password_in_the_request_lets_a_protected_pdf_through_under_headless() {
    let step = a_consent_over(
        A_PASSWORD_PROTECTED_PDF,
        &format!("headless=true\nuserPassword={THE_PASSWORD}\n"),
    );

    let ErrandStep::AskingToSign(consent) = step else {
        panic!("la peticion trae la contrasena: {step:?}");
    };
    assert_eq!(
        consent
            .from_the_site
            .get("userPassword")
            .map(String::as_str),
        Some(THE_PASSWORD),
        "el puente abre el PDF con ella"
    );
}

#[test]
fn allow_cosigning_unregistered_signatures_lets_the_pdf_through_under_headless() {
    let step = a_consent_over(
        A_PDF_SIGNED_BY_SOMETHING_ELSE,
        "headless=true\nallowCosigningUnregisteredSignatures=true\n",
    );

    assert!(
        matches!(step, ErrandStep::AskingToSign(_)),
        "la sede ya lo permitio: {step:?}"
    );
}

#[test]
fn the_password_never_reaches_the_refusal() {
    let step = a_consent_over(
        A_CERTIFIED_PDF,
        &format!("headless=true\nuserPassword={THE_PASSWORD}\n"),
    );

    let ErrandStep::Answering(SiteOutcome::Refused(refusal)) = &step else {
        panic!("sigue certificado: {step:?}");
    };
    let (failure, _) = frontier::told(refusal);
    assert!(!format!("{refusal:?}{}", failure.detail).contains(THE_PASSWORD));
}
