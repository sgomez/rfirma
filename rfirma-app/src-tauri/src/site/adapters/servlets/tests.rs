use super::*;

#[test]
fn a_malformed_url_fails_without_panicking() {
    assert!(parsed_servlet_url("no es una url").is_err());
}

#[test]
fn a_retrieve_composes_op_v_id_in_that_order() {
    assert_eq!(
        operation_params("get", "abc123", None),
        vec![("op", "get"), ("v", "1_0"), ("id", "abc123")]
    );
}

#[test]
fn a_store_appends_dat_last() {
    assert_eq!(
        operation_params("put", "abc123", Some("firma-en-base64")),
        vec![
            ("op", "put"),
            ("v", "1_0"),
            ("id", "abc123"),
            ("dat", "firma-en-base64")
        ]
    );
}

#[test]
fn the_wait_marker_matches_the_original_byte_for_byte() {
    assert_eq!(WAIT_MARKER, "#WAIT");
}

#[tokio::test(flavor = "multi_thread")]
async fn store_from_within_an_async_tokio_context_does_not_panic() {
    let servlets = RelayServlets::default();
    let result = servlets.store(
        "https://unreachable.invalid/afirma/StorageService",
        "id-123",
        "data-abc",
    );
    assert!(matches!(
        result,
        Err(error) if error.situation() == Situation::ServletUnreachable
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn retrieve_from_within_an_async_tokio_context_does_not_panic() {
    let servlets = RelayServlets::default();
    let result = servlets.retrieve(
        "https://unreachable.invalid/afirma/RetrieveService",
        "id-123",
    );
    assert!(matches!(
        result,
        Err(error) if error.situation() == Situation::ServletUnreachable
    ));
}

#[test]
fn the_url_credentials_travel_as_a_basic_authorization_header() {
    use crate::site::adapters::header_probe::{authorization_probe, with_alice, ALICE_BASIC};
    let (address, received) = authorization_probe();
    let url = with_alice(&address);

    let _ = RelayServlets::default().retrieve(&url, "id-123");

    assert_eq!(
        received.recv().expect("llego la peticion").as_deref(),
        Some(ALICE_BASIC)
    );
}

#[test]
fn a_url_without_credentials_sends_no_authorization_header() {
    use crate::site::adapters::header_probe::{authorization_probe, without_credentials};
    let (address, received) = authorization_probe();
    let url = without_credentials(&address);

    let _ = RelayServlets::default().retrieve(&url, "id-123");

    assert_eq!(received.recv().expect("llego la peticion"), None);
}

#[test]
fn a_rejected_retrieve_keeps_the_body_of_the_error() {
    let url = crate::site::adapters::header_probe::rejecting_server(
        "404 Not Found",
        "ERR-01: documento desconocido",
    );

    let error = RelayServlets::default()
        .retrieve(&url, "id-1")
        .expect_err("el servidor rechaza");

    assert!(
        error.to_string().contains("ERR-01: documento desconocido"),
        "{error}"
    );
}

#[test]
fn a_rejected_store_keeps_the_body_of_the_error() {
    let url = crate::site::adapters::header_probe::rejecting_server(
        "404 Not Found",
        "ERR-01: documento desconocido",
    );

    let error = RelayServlets::default()
        .store(&url, "id-1", "dat")
        .expect_err("el servidor rechaza");

    assert!(
        error.to_string().contains("ERR-01: documento desconocido"),
        "{error}"
    );
}
