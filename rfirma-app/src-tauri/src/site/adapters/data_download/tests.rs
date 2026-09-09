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
