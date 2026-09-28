use reqwest::cookie::CookieStore;

use super::*;
use crate::site::adapters::cookies::OperationCookies;

fn site_url() -> url::Url {
    url::Url::parse("https://relay.example/retrieve").expect("URL valida")
}

fn a_cookie_in(cookies: &OperationCookies) {
    let header = reqwest::header::HeaderValue::from_static("JSESSIONID=abc123; Path=/");
    cookies.set_cookies(&mut std::iter::once(&header), &site_url());
}

fn a_relay_with_cookies(servlets: Arc<OrderedSpy>) -> (Relay, Spy, Arc<OperationCookies>) {
    let (mut relay, spy) = a_relay(servlets);
    let cookies: Arc<OperationCookies> = Arc::default();
    relay.cookies = Arc::clone(&cookies);
    (relay, spy, cookies)
}

#[test]
fn a_refusal_leaves_no_cookies_for_the_next_invocation() {
    let (relay, _spy, cookies) = a_relay_with_cookies(Arc::new(OrderedSpy::default()));
    a_cookie_in(&cookies);
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(a_key()), true));
    let answer = Refusal::new(SafCode::CannotOpenSocket, "ya hay un tramite vivo").answer();

    relay
        .open(&info, ChannelDuty::Refuse(answer))
        .expect("abre con la entrega pendiente");

    assert!(cookies.cookies(&site_url()).is_none());
}

#[test]
fn a_failed_resolve_leaves_no_cookies_for_the_next_invocation() {
    let (relay, _spy, cookies) = a_relay_with_cookies(Arc::new(OrderedSpy::default()));
    a_cookie_in(&cookies);
    let info = ChannelLocation::Relay(a_parameters_info("fileid-inexistente", Some(a_key())));

    relay
        .open(&info, duty())
        .expect_err("no hay nada que recuperar");

    assert!(cookies.cookies(&site_url()).is_none());
}

#[test]
fn answering_the_operation_forgets_its_cookies() {
    let servlets = Arc::new(OrderedSpy::default());
    let (relay, spy, cookies) = a_relay_with_cookies(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, None, false));
    servlets
        .store(RETRIEVE_SERVLET, "fileid-1", "AAAA")
        .expect("guarda el documento");
    opened_and_delivered(&relay, &info);
    a_cookie_in(&cookies);

    let (_operation, reply) = spy.take_reply();
    reply.answer("la-respuesta".to_owned());

    assert!(cookies.cookies(&site_url()).is_none());
}

#[test]
fn an_invocation_refused_for_an_errand_in_flight_keeps_the_live_errand_cookies() {
    let servlets = Arc::new(OrderedSpy::default());
    let (relay, _spy, cookies) = a_relay_with_cookies(Arc::clone(&servlets));
    let relay = relay.minding_the_live_errand(|| true);
    a_cookie_in(&cookies);
    servlets
        .store(RETRIEVE_SERVLET, "fileid-1", "AAAA")
        .expect("guarda el documento");
    let serving = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, None, false));
    let refusing = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(a_key()), true));
    let answer = Refusal::new(SafCode::CannotOpenSocket, "ya hay un tramite vivo").answer();

    relay
        .open(&serving, duty())
        .expect("resuelve la operacion")
        .close();
    relay
        .open(&refusing, ChannelDuty::Refuse(answer))
        .expect("abre con la entrega pendiente");

    assert!(cookies.cookies(&site_url()).is_some());
}
