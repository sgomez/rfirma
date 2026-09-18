use super::*;

#[test]
fn discussions_identifier_resolves_to_discussions_url() {
    assert_eq!(
        resolve_destination(DISCUSSIONS),
        Some("https://github.com/sgomez/rfirma/discussions")
    );
}

#[test]
fn unknown_identifier_is_rejected() {
    assert_eq!(resolve_destination("unknown"), None);
    assert_eq!(resolve_destination("issues"), None);
    assert_eq!(resolve_destination(""), None);
}

#[test]
fn url_cannot_be_passed_as_destination() {
    assert_eq!(
        resolve_destination("https://github.com/sgomez/rfirma/discussions"),
        None
    );
    assert_eq!(resolve_destination("https://example.com"), None);
}
