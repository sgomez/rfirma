use super::*;
use crate::signing::domain::bridge::XadesVariant;

const AN_XML: &[u8] = b"<?xml version=\"1.0\"?><raiz>datos</raiz>";
const A_BINARY: &[u8] = b"\x00\x01 no es un PDF ni un XML";

fn format_asked_for(words: &[&str], input: &[u8]) -> SignatureFormat {
    let files = FilesInMemory::with("entrada", input);
    let signer = RecordingSigner::default();
    let mut all = vec!["sign", "-i", "entrada", "-o", "salida", "-alias", "yo"];
    all.extend_from_slice(words);

    let outcome = signed_over(&all, &files, &signer);

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    assert_eq!(files.at("salida").as_deref(), Some(SIGNED));
    let asked = signer.asked.borrow();
    asked[0].2
}

#[test]
fn sign_cades_signs_any_file_in_cades() {
    for input in [A_BINARY, AN_XML, A_PDF] {
        assert_eq!(
            format_asked_for(&["-format", "cades"], input),
            SignatureFormat::Cades
        );
    }
}

#[test]
fn sign_xades_signs_any_file_in_xades_with_the_default_envelope_of_the_original() {
    for input in [A_BINARY, AN_XML, A_PDF] {
        assert_eq!(
            format_asked_for(&["-format", "xades"], input),
            SignatureFormat::Xades(XadesVariant::Enveloping)
        );
    }
}

#[test]
fn sign_auto_picks_pades_for_a_pdf_xades_for_an_xml_and_cades_for_anything_else() {
    for (input, expected) in [
        (A_PDF, SignatureFormat::Pades),
        (AN_XML, SignatureFormat::Xades(XadesVariant::Enveloping)),
        (A_BINARY, SignatureFormat::Cades),
    ] {
        assert_eq!(format_asked_for(&["-format", "auto"], input), expected);
        assert_eq!(format_asked_for(&[], input), expected);
    }
}

#[test]
fn sign_xades_hands_the_signer_the_envelope_declared_in_config() {
    let files = FilesInMemory::with("datos.bin", A_BINARY);
    let signer = RecordingSigner::default();

    let outcome = signed_over(
        &[
            "sign",
            "-i",
            "datos.bin",
            "-o",
            "f.xsig",
            "-alias",
            "yo",
            "-format",
            "xades",
            "-config",
            "format=XAdES Detached",
        ],
        &files,
        &signer,
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    let expected = BTreeMap::from([("format".to_owned(), "XAdES Detached".to_owned())]);
    assert_eq!(*signer.parameters.borrow(), vec![expected]);
}
