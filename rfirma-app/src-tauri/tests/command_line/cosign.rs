//! Las pruebas de grada C de `cosign` en los tres formatos con el puente real.

use super::*;
use rfirma_lib::desktop::ports::SignatureVerifier;
use rfirma_lib::signing::domain::bridge::XadesVariant;

const A_TEXT: &[u8] = b"Documento de prueba que no es un PDF\n";
const AN_XML: &[u8] =
    b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<documento>Documento de prueba</documento>\n";

fn utf8(path: &Path) -> String {
    path.to_str().expect("ruta UTF-8").to_owned()
}

fn run(roots: &Roots, command: &str, files: [&Path; 2], alias: &str, extra: &[&str]) {
    let mut words = vec![
        command.to_owned(),
        "-i".to_owned(),
        utf8(files[0]),
        "-o".to_owned(),
        utf8(files[1]),
        "-alias".to_owned(),
        alias.to_owned(),
    ];
    words.extend(extra.iter().map(|word| (*word).to_owned()));
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let outcome = attended_with_the_roots(&words, roots, &ScriptedTerminal::without_a_tty());
    assert_eq!(
        outcome.exit_code, SUCCEEDED,
        "{command}: {:?}",
        outcome.stderr
    );
}

fn cosigned(document: &[u8], extra: &[&str]) -> (Roots, Vec<u8>, tempfile::TempDir) {
    let (home, alias) = a_home_with_an_installed_certificate();
    let roots = the_roots_under(home.path());
    let [original, first, second] =
        ["documento", "primera", "segunda"].map(|name| home.path().join(name));
    std::fs::write(&original, document).expect("el documento deberia escribirse");
    run(&roots, "sign", [&original, &first], &alias, extra);
    run(&roots, "cosign", [&first, &second], &alias, extra);
    let signed = std::fs::read(&second).expect("la cofirma deberia estar en -o");
    (roots, signed, home)
}

fn occurrences_of(needle: &str, signed: &[u8]) -> usize {
    String::from_utf8_lossy(signed).matches(needle).count()
}

fn assert_two_valid_signatures(roots: &Roots, signed: &[u8], format: Format) {
    let signatures = match format {
        Format::Pades => occurrences_of("/ByteRange", signed),
        Format::Xades(_) => occurrences_of("<ds:Signature ", signed),
        _ => NativeVerifier
            .results_of(signed, format)
            .expect("el validador del puente deberia contestar")
            .len(),
    };
    assert_eq!(signatures, 2);
    assert_eq!(
        super::formats::verdict_in(roots, signed, format),
        SignatureVerdict::Valid
    );
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn cosign_on_a_signed_pdf_adds_a_second_valid_pades_signature() {
    let (roots, signed, _home) = cosigned(&a_one_page_pdf(), &[]);

    assert_two_valid_signatures(&roots, &signed, Format::Pades);
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn cosign_on_a_cades_signature_adds_a_second_valid_signature() {
    let (roots, signed, _home) =
        cosigned(A_TEXT, &["-format", "cades", "-config", "mode=implicit"]);

    assert_two_valid_signatures(&roots, &signed, Format::Cades);
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn cosign_on_an_xades_signature_adds_a_second_valid_signature() {
    let (roots, signed, _home) = cosigned(AN_XML, &["-format", "xades"]);

    assert_two_valid_signatures(&roots, &signed, Format::Xades(XadesVariant::Enveloping));
}
