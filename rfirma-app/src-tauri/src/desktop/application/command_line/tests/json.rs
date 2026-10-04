use serde_json::{json, Value};

use super::*;

/// Lo que `--json` debe decir de una respuesta XML plana, según la regla de traducción.
fn translated(xml: &str, repeatable: &[&str]) -> Value {
    let unescaped = |text: &str| {
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
    };
    let inner = xml
        .trim_end()
        .strip_prefix("<afirma>")
        .and_then(|rest| rest.strip_suffix("</afirma>"))
        .expect("la raíz es afirma");
    let (result, response) = inner
        .strip_prefix("<result>")
        .and_then(|rest| rest.split_once("</result><response>"))
        .expect("el resultado va primero");
    let mut fields = serde_json::Map::new();
    let mut rest = response
        .strip_suffix("</response>")
        .expect("response cierra");
    for name in repeatable {
        fields.insert((*name).to_owned(), json!([]));
    }
    while let Some(open) = rest.strip_prefix('<') {
        let (name, after) = open.split_once('>').expect("etiqueta abierta");
        let (text, next) = after
            .split_once(&format!("</{name}>"))
            .expect("etiqueta cerrada");
        let value = Value::String(unescaped(text));
        if repeatable.contains(&name) {
            fields[name].as_array_mut().expect("lista").push(value);
        } else {
            fields.insert(name.to_owned(), value);
        }
        rest = next;
    }
    json!({"afirma": {"result": result, "response": fields}})
}

fn json_of(outcome: &Outcome) -> Value {
    serde_json::from_slice(&outcome.stdout).expect("stdout es JSON")
}

fn same_information_in_both(run: impl Fn(&str) -> Outcome, repeatable: &[&str]) {
    let xml = run("-xml");
    let json = run("-json");

    assert_eq!(json.exit_code, xml.exit_code);
    assert_eq!(json.stderr, xml.stderr);
    let xml_text = String::from_utf8(xml.stdout).expect("UTF-8");
    assert_eq!(json_of(&json), translated(&xml_text, repeatable));
}

fn signing(flag: &str, signer: &RecordingSigner) -> Outcome {
    signed_over(
        &["sign", "-i", "doc.pdf", "-alias", "yo", flag],
        &FilesInMemory::with("doc.pdf", A_PDF),
        signer,
    )
}

#[test]
fn sign_json_says_what_sign_xml_says_on_success() {
    same_information_in_both(|flag| signing(flag, &RecordingSigner::default()), &[]);
}

#[test]
fn sign_json_says_what_sign_xml_says_on_failure() {
    let signer = RecordingSigner {
        fails: true,
        ..RecordingSigner::default()
    };

    same_information_in_both(|flag| signing(flag, &signer), &[]);

    let outcome = signing("-json", &signer);
    assert_eq!(outcome.exit_code, FAILED);
    assert_eq!(json_of(&outcome)["afirma"]["result"], "false");
}

#[test]
fn cosign_json_carries_the_signature_as_a_string() {
    let outcome = signed_over(
        &["cosign", "-i", "doc.pdf", "-alias", "yo", "-json"],
        &FilesInMemory::with("doc.pdf", A_PDF),
        &RecordingSigner::default(),
    );

    assert_eq!(outcome.exit_code, SUCCEEDED, "{}", said(&outcome));
    assert!(json_of(&outcome)["afirma"]["response"]["sign"].is_string());
}

#[test]
fn json_with_xml_is_refused_whatever_the_command() {
    for words in [
        &["listaliases", "-json", "-xml"][..],
        &["sign", "-i", "doc.pdf", "-alias", "yo", "-xml", "-json"],
    ] {
        let outcome = attended(words);

        assert_eq!(outcome.exit_code, REFUSED, "{}", said(&outcome));
        assert!(outcome.stdout.is_empty());
    }
}

#[test]
fn verify_json_is_refused_like_verify_xml() {
    let outcome = attended(&["verify", "-i", "firmado.pdf", "-json"]);

    assert_eq!(outcome.exit_code, FAILED);
    assert!(outcome.stdout.is_empty());
}

#[test]
fn a_message_with_quotes_backslash_and_control_characters_comes_out_escaped() {
    let input = "a\"b\\c\u{1}.pdf";

    let outcome = signed_over(
        &["sign", "-i", input, "-o", "f.pdf", "-alias", "yo", "-json"],
        &FilesInMemory::with("doc.pdf", A_PDF),
        &RecordingSigner::default(),
    );

    assert_eq!(outcome.exit_code, FAILED);
    let response = json_of(&outcome);
    let message = response["afirma"]["response"]["msg"]
        .as_str()
        .expect("msg es una cadena");
    assert!(message.contains(input), "{message}");
}
