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
