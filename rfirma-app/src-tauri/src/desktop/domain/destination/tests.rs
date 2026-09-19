use super::*;

#[test]
fn discussions_identifier_resolves_to_discussions_url() {
    assert_eq!(
        resolve_destination(DISCUSSIONS),
        Some("https://github.com/sgomez/rfirma/discussions")
    );
}

#[test]
fn releases_identifier_resolves_to_releases_url() {
    assert_eq!(
        resolve_destination(RELEASES),
        Some("https://github.com/sgomez/rfirma/releases")
    );
}

#[test]
fn repository_identifier_resolves_to_repository_url() {
    assert_eq!(
        resolve_destination(REPOSITORY),
        Some("https://rfirma.sgomez.me/")
    );
}

#[test]
fn certificate_issuance_identifier_resolves_to_the_fnmt_url() {
    assert_eq!(
        resolve_destination(CERTIFICATE_ISSUANCE),
        Some(
            "https://www.sede.fnmt.gob.es/certificados/persona-fisica/obtener-certificado-software"
        )
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
