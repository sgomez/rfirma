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
