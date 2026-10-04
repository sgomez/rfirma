use super::*;

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
