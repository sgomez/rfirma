use super::*;

#[test]
fn a_host_that_does_not_resolve_comes_back_as_a_detail_and_not_as_a_panic() {
    let failure = HttpDataSource
        .download("https://unreachable.invalid/documentos/4711.pdf")
        .expect_err("ese dominio no existe");

    assert!(
        !failure.is_empty(),
        "el detalle dice por que no se ha bajado"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_download_from_within_an_async_tokio_context_does_not_panic() {
    let failure = HttpDataSource
        .download("https://unreachable.invalid/documentos/4711.pdf")
        .expect_err("ese dominio no existe");

    assert!(
        !failure.is_empty(),
        "el detalle dice por que no se ha bajado"
    );
}

#[test]
fn the_url_credentials_travel_as_a_basic_authorization_header() {
    use crate::site::adapters::header_probe::{authorization_probe, with_alice, ALICE_BASIC};
    let (address, received) = authorization_probe();
    let url = with_alice(&address);

    let _ = HttpDataSource.download(&url);

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

    let _ = HttpDataSource.download(&url);

    assert_eq!(received.recv().expect("llego la peticion"), None);
}

#[test]
fn a_rejected_download_keeps_the_body_of_the_error() {
    let url = crate::site::adapters::header_probe::rejecting_server(
        "404 Not Found",
        "ERR-01: documento desconocido",
    );

    let error = HttpDataSource
        .download(&url)
        .expect_err("el servidor rechaza");

    assert!(
        error.to_string().contains("ERR-01: documento desconocido"),
        "{error}"
    );
}

#[test]
fn a_download_that_answers_after_the_connect_limit_still_arrives() {
    use crate::site::adapters::header_probe::{slow_server, BEYOND_CONNECT_LIMIT};
    let url = slow_server(BEYOND_CONNECT_LIMIT, "contenido");

    let bytes = HttpDataSource.download(&url).expect("el servidor contesta");

    assert_eq!(bytes, b"contenido");
}

#[test]
fn an_untrusted_server_certificate_is_named_as_such_and_stays_unreachable() {
    let url = crate::site::adapters::header_probe::untrusted_tls_server();

    let error = HttpDataSource
        .download(&url)
        .expect_err("el certificado no es de confianza");

    assert!(
        error
            .to_string()
            .contains("certificado del servidor 127.0.0.1 no es de confianza"),
        "{error}"
    );
}
