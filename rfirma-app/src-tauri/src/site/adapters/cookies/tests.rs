use std::sync::Arc;

use super::OperationCookies;
use crate::site::adapters::batch_services::RelayBatchServices;
use crate::site::adapters::data_download::download_with;
use crate::site::adapters::header_probe::cookie_probe;
use crate::site::adapters::triphase_server::HttpTriphaseServer;
use crate::site::domain::batch::BatchFormat;
use crate::site::ports::{BatchServices, TriphaseServer};

const SESSION: &str = "JSESSIONID=abc123; Path=/";

#[test]
fn the_presign_cookie_travels_in_the_next_call_of_the_batch() {
    let (presign, _) = cookie_probe(SESSION);
    let (postsign, postsign_cookie) = cookie_probe("OTHER=1; Path=/");
    let services = RelayBatchServices::with_cookies(Arc::default());

    services
        .presign(&presign, BatchFormat::Json, "bG90ZQ", &[])
        .expect("la prefirma contesta");
    services
        .presign(&postsign, BatchFormat::Json, "bG90ZQ", &[])
        .expect("la segunda llamada contesta");

    assert_eq!(
        postsign_cookie.recv().unwrap().as_deref(),
        Some("JSESSIONID=abc123")
    );
}

#[test]
fn a_cookie_one_client_receives_reaches_the_other_clients_of_the_operation() {
    let cookies: Arc<OperationCookies> = Arc::default();
    let (download, _) = cookie_probe(SESSION);
    let (server, server_cookie) = cookie_probe("OTHER=1; Path=/");

    download_with(Arc::clone(&cookies), &download).expect("la descarga contesta");
    HttpTriphaseServer::with_cookies(cookies)
        .post(&server, &[])
        .expect("el servidor contesta");

    assert_eq!(
        server_cookie.recv().unwrap().as_deref(),
        Some("JSESSIONID=abc123")
    );
}

#[test]
fn the_next_operation_does_not_send_the_cookies_of_the_previous_one() {
    let cookies: Arc<OperationCookies> = Arc::default();
    let (first, _) = cookie_probe(SESSION);
    let (second, second_cookie) = cookie_probe("OTHER=1; Path=/");
    let services = RelayBatchServices::with_cookies(Arc::clone(&cookies));

    services
        .presign(&first, BatchFormat::Json, "bG90ZQ", &[])
        .expect("la prefirma contesta");
    cookies.forget();
    services
        .presign(&second, BatchFormat::Json, "bG90ZQ", &[])
        .expect("la segunda operacion contesta");

    assert_eq!(second_cookie.recv().unwrap(), None);
}
