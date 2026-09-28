use super::*;

#[test]
fn a_plain_https_origin_keeps_its_host() {
    let origin = SiteOrigin::from_header(Some("https://sede.ejemplo.gob.es"));
    assert_eq!(origin.host(), Some("sede.ejemplo.gob.es"));
}

#[test]
fn a_non_standard_port_travels_with_the_host() {
    let origin = SiteOrigin::from_header(Some("https://sede.ejemplo.gob.es:8443"));
    assert_eq!(origin.host(), Some("sede.ejemplo.gob.es:8443"));
}

#[test]
fn a_punycode_host_is_not_decoded() {
    let origin = SiteOrigin::from_header(Some("https://xn--sede-2sa.example"));
    assert_eq!(origin.host(), Some("xn--sede-2sa.example"));
}

#[test]
fn a_missing_header_is_absent() {
    let origin = SiteOrigin::from_header(None);
    assert_eq!(origin.host(), None);
}

#[test]
fn the_literal_null_is_absent() {
    let origin = SiteOrigin::from_header(Some("null"));
    assert_eq!(origin.host(), None);
}

#[test]
fn a_plain_http_origin_is_absent() {
    let origin = SiteOrigin::from_header(Some("http://sede.ejemplo.gob.es"));
    assert_eq!(origin.host(), None);
}

#[test]
fn an_origin_with_a_path_is_absent() {
    let origin = SiteOrigin::from_header(Some("https://sede.ejemplo.gob.es/camino"));
    assert_eq!(origin.host(), None);
}

#[test]
fn an_origin_with_a_query_is_absent() {
    let origin = SiteOrigin::from_header(Some("https://sede.ejemplo.gob.es?a=b"));
    assert_eq!(origin.host(), None);
}

#[test]
fn an_origin_with_credentials_is_absent() {
    let origin = SiteOrigin::from_header(Some("https://persona:secreto@sede.ejemplo.gob.es"));
    assert_eq!(origin.host(), None);
}

#[test]
fn an_empty_host_is_absent() {
    let origin = SiteOrigin::from_header(Some("https://"));
    assert_eq!(origin.host(), None);
}

#[test]
fn an_unparseable_value_is_absent() {
    let origin = SiteOrigin::from_header(Some("no es una url"));
    assert_eq!(origin.host(), None);
}

#[test]
fn an_empty_host_before_the_port_is_absent() {
    let origin = SiteOrigin::from_header(Some("https://:8443"));
    assert_eq!(origin.host(), None);
}

#[test]
fn a_host_with_a_space_is_absent() {
    let origin = SiteOrigin::from_header(Some("https://sede ejemplo.gob.es"));
    assert_eq!(origin.host(), None);
}

#[test]
fn a_non_numeric_port_is_absent() {
    let origin = SiteOrigin::from_header(Some("https://sede.gob.es:abc"));
    assert_eq!(origin.host(), None);
}

#[test]
fn a_trailing_colon_with_no_port_is_absent() {
    let origin = SiteOrigin::from_header(Some("https://sede.gob.es:"));
    assert_eq!(origin.host(), None);
}

#[test]
fn the_absent_constructor_has_no_host() {
    assert_eq!(SiteOrigin::absent().host(), None);
}
