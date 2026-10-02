//! Las pruebas de grada C de `sign -format cades` y `-format xades` con el puente real.

use super::*;
use rfirma_lib::signing::domain::bridge::XadesVariant;

const A_TEXT: &[u8] = b"Documento de prueba que no es un PDF\n";
const AN_XML: &[u8] =
    b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<documento>Documento de prueba</documento>\n";

fn signed_as(input_bytes: &[u8], extra: &[&str]) -> (tempfile::TempDir, Roots, Outcome, PathBuf) {
    let (home, installed_alias) = a_home_with_an_installed_certificate();
    let roots = the_roots_under(home.path());
    let input = home.path().join("documento.txt");
    std::fs::write(&input, input_bytes).expect("el documento deberia escribirse");
    let output = home.path().join("firma");
    let mut words = vec![
        "sign",
        "-i",
        input.to_str().expect("ruta UTF-8"),
        "-o",
        output.to_str().expect("ruta UTF-8"),
        "-alias",
        &installed_alias,
    ];
    words.extend_from_slice(extra);
    let outcome = attended_with_the_roots(&words, &roots, &ScriptedTerminal::without_a_tty());
    (home, roots, outcome, output)
}

pub(super) fn verdict_in(roots: &Roots, signed: &[u8], format: Format) -> SignatureVerdict {
    roots
        .signing
        .isolate
        .verdict_of(
            &base64::engine::general_purpose::STANDARD.encode(signed),
            format,
        )
        .expect("el validador del puente deberia contestar")
}

fn the_valid_signature_of(input: &[u8], extra: &[&str], format: Format) -> String {
    let (_home, roots, outcome, output) = signed_as(input, extra);

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    let signed = std::fs::read(&output).expect("la firma deberia estar en -o");
    assert_eq!(verdict_in(&roots, &signed, format), SignatureVerdict::Valid);
    String::from_utf8_lossy(&signed).into_owned()
}

fn root_element_of(xml: &str) -> &str {
    xml.split('<')
        .find(|tag| !tag.starts_with('?') && !tag.is_empty())
        .and_then(|tag| tag.split([' ', '>']).next())
        .unwrap_or_default()
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn sign_cades_and_auto_sign_a_file_that_is_not_a_pdf_with_a_valid_cades() {
    for extra in [
        &["-format", "cades", "-config", "mode=implicit"][..],
        &["-config", "mode=implicit"][..],
    ] {
        the_valid_signature_of(A_TEXT, extra, Format::Cades);
    }
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn sign_cades_without_config_leaves_the_document_out_like_the_original() {
    let (_home, roots, outcome, output) = signed_as(A_TEXT, &["-format", "cades"]);

    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    let signed = std::fs::read(&output).expect("la firma deberia estar en -o");
    assert_eq!(
        verdict_in(&roots, &signed, Format::Cades),
        SignatureVerdict::Invalid {
            reason: "NO_DATA".to_owned()
        }
    );
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn sign_xades_signs_a_file_that_is_not_a_pdf_with_a_valid_enveloping_xades() {
    let signed = the_valid_signature_of(
        A_TEXT,
        &["-format", "xades"],
        Format::Xades(XadesVariant::Enveloping),
    );

    assert_eq!(root_element_of(&signed), "ds:Signature", "{signed}");
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn sign_xades_takes_the_envelope_declared_in_config() {
    let signed = the_valid_signature_of(
        AN_XML,
        &["-format", "xades", "-config", "format=XAdES Detached"],
        Format::Xades(XadesVariant::Detached),
    );

    assert_ne!(root_element_of(&signed), "ds:Signature", "{signed}");
}
