use base64::Engine as _;
use serde_json::{json, Value};

use super::schema::conforming_json;
use super::*;
use crate::identity::application::tests::TestAuthority;

const AN_XML: &[u8] = b"<?xml version=\"1.0\"?><raiz>datos</raiz>";
const A_BINARY: &[u8] = b"\x00\x01 no es un PDF ni un XML";

struct StoresHolding(Vec<TokenCertificate>);

impl CertificateStores for StoresHolding {
    fn certificates(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(self.0.clone())
    }

    fn discovered_module(&self, _library: &str) -> Option<PathBuf> {
        None
    }
}

fn signed_as_json(command: &str, input: &[u8], extra: &[&str]) -> Value {
    let mut words = vec![command, "-i", "entrada", "-alias", "yo", "-json"];
    words.extend_from_slice(extra);
    let outcome = signed_over(
        &words,
        &FilesInMemory::with("entrada", input),
        &RecordingSigner::default(),
    );
    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    conforming_json(command, &outcome.stdout)
}

#[test]
fn sign_json_with_an_output_gives_the_format_the_certificate_and_the_absolute_path() {
    let signed = signed_as_json("sign", A_PDF, &["-o", "firmado.pdf"]);

    assert_eq!(signed["format"], "PAdES");
    assert_eq!(signed["certificate"], json!({"alias": "yo"}));
    let output = signed["output"].as_str().expect("output es una cadena");
    assert!(Path::new(output).is_absolute(), "{output}");
    assert!(output.ends_with("firmado.pdf"), "{output}");
    assert!(signed.get("signature").is_none());
}

#[test]
fn sign_json_without_an_output_carries_the_signature_in_standard_base64() {
    let signed = signed_as_json("sign", A_PDF, &[]);

    let expected = base64::engine::general_purpose::STANDARD.encode(SIGNED);
    assert_eq!(signed["signature"], expected.as_str());
    assert!(signed.get("output").is_none());
}

#[test]
fn sign_json_says_the_resolved_format_also_when_auto_was_asked() {
    for (input, expected) in [(A_PDF, "PAdES"), (AN_XML, "XAdES"), (A_BINARY, "CAdES")] {
        for extra in [&["-format", "auto"][..], &[]] {
            let signed = signed_as_json("sign", input, extra);

            assert_eq!(signed["format"], expected);
        }
    }
}

#[test]
fn sign_json_names_the_formats_asked_for_with_their_official_names() {
    for (format, expected) in [("pades", "PAdES"), ("cades", "CAdES"), ("xades", "XAdES")] {
        let signed = signed_as_json("sign", A_BINARY, &["-format", format]);

        assert_eq!(signed["format"], expected);
    }
}

#[test]
fn sign_json_gives_the_common_fields_of_the_certificate_with_its_alias() {
    let root = TestAuthority::root_with_serial("Raíz de pruebas", 0x0A_1B2C);
    let (not_before, not_after) = root.as_certificate("yo").validity().expect("el DER se lee");
    let utc = |instant: std::time::SystemTime| {
        chrono::DateTime::<chrono::Utc>::from(instant)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    };

    let outcome = attended_in(
        &[
            "sign", "-i", "doc.pdf", "-o", "f.pdf", "-alias", "yo", "-json",
        ],
        &StoresHolding(vec![root.as_certificate("yo")]),
        &RecordingDesktop::default(),
        &FilesInMemory::with("doc.pdf", A_PDF),
        &RecordingSigner::default(),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    let signed = conforming_json("sign", &outcome.stdout);
    assert_eq!(
        signed["certificate"],
        json!({
            "alias": "yo",
            "subject": "CN=Raíz de pruebas",
            "issuer": "CN=Raíz de pruebas",
            "serialNumber": "0A1B2C",
            "notBefore": utc(not_before),
            "notAfter": utc(not_after),
        })
    );
}

#[test]
fn cosign_json_has_the_same_shape_as_sign_json() {
    let with_output = signed_as_json("cosign", A_PDF, &["-o", "dos.pdf"]);
    let without_output = signed_as_json("cosign", A_PDF, &[]);

    assert_eq!(with_output["format"], "PAdES");
    assert!(with_output["output"].is_string());
    assert!(without_output["signature"].is_string());
}

#[test]
fn sign_json_that_fails_leaves_stdout_empty_and_says_why_on_stderr() {
    let signer = RecordingSigner {
        fails: true,
        ..RecordingSigner::default()
    };
    let files = FilesInMemory::with("doc.pdf", A_PDF);

    for command in ["sign", "cosign"] {
        let signing_fails = signed_over(
            &[
                command, "-i", "doc.pdf", "-o", "f.pdf", "-alias", "yo", "-json",
            ],
            &files,
            &signer,
        );
        let no_such_alias = signed_over(
            &[command, "-i", "doc.pdf", "-alias", "nadie", "-json"],
            &files,
            &RecordingSigner::default(),
        );
        let no_such_input = signed_over(
            &[command, "-i", "falta.pdf", "-alias", "yo", "-json"],
            &files,
            &RecordingSigner::default(),
        );

        for outcome in [signing_fails, no_such_alias, no_such_input] {
            assert_ne!(outcome.exit_code, SUCCEEDED);
            assert!(outcome.stdout.is_empty(), "{:?}", outcome.stdout);
            assert!(!outcome.stderr.is_empty());
        }
    }
}
