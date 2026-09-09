use super::*;

#[test]
fn an_https_servlet_url_is_accepted() {
    assert!(validated_servlet_url("https://servlet.example/afirma/StorageService").is_ok());
}

#[test]
fn an_http_servlet_url_is_accepted_like_the_original() {
    assert!(validated_servlet_url("http://servlet.example/afirma/StorageService").is_ok());
}

#[test]
fn a_malformed_url_fails_without_panicking() {
    assert!(validated_servlet_url("no es una url").is_err());
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
