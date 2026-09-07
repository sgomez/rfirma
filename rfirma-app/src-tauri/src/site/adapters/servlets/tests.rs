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
fn an_unsupported_protocol_is_rejected() {
    let result = validated_servlet_url("ftp://servlet.example/afirma/StorageService");

    assert!(matches!(
        result,
        Err(error) if error.situation() == Situation::ServletUnreachable
    ));
}

#[test]
fn localhost_is_rejected_as_a_local_access_attempt() {
    assert!(validated_servlet_url("https://localhost/afirma/StorageService").is_err());
    assert!(validated_servlet_url("https://127.0.0.1/afirma/StorageService").is_err());
}

#[test]
fn a_url_carrying_its_own_query_parameters_is_rejected() {
    assert!(validated_servlet_url("https://servlet.example/StorageService?op=get").is_err());
    assert!(validated_servlet_url("https://servlet.example/afirma/StorageService").is_ok());
}

#[test]
fn a_malformed_url_fails_without_panicking() {
    assert!(validated_servlet_url("no es una url").is_err());
}
