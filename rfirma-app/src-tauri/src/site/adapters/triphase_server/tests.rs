use super::*;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

/// Un servidor HTTP de una sola petición en el bucle local: contesta `status` con `body` y devuelve lo que recibió.
fn one_request_answered(status: &str, body: &'static str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("hay un puerto libre");
    let url = format!(
        "http://{}/tri",
        listener.local_addr().expect("tiene direccion")
    );
    let status = status.to_owned();
    let (sender, received) = mpsc::channel();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("llega la peticion");
        let mut reader = BufReader::new(stream);
        let mut head = String::new();
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("se lee la cabecera");
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().expect("es un numero");
            }
            head.push_str(&line);
            if line == "\r\n" {
                break;
            }
        }
        let mut request_body = vec![0; length];
        reader
            .read_exact(&mut request_body)
            .expect("se lee el cuerpo");
        let answer = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        reader
            .get_mut()
            .write_all(answer.as_bytes())
            .expect("se contesta");
        sender
            .send(head + &String::from_utf8_lossy(&request_body))
            .expect("la prueba escucha");
    });
    (url, received)
}

#[test]
fn the_form_travels_in_the_body_of_a_post_and_the_answer_comes_back_raw() {
    let (url, received) = one_request_answered("200 OK", "OK NEWID=ZmlybWE=");

    let answer = HttpTriphaseServer::default()
        .post(
            &url,
            &[("op", "pre".to_owned()), ("doc", "ZGF0b3M=".to_owned())],
        )
        .expect("el servidor contesta");

    let request = received.recv().expect("llego la peticion");
    assert!(request.starts_with("POST /tri "), "{request}");
    assert!(request.ends_with("op=pre&doc=ZGF0b3M%3D"), "{request}");
    assert_eq!(answer, b"OK NEWID=ZmlybWE=");
}

#[test]
fn an_http_error_of_the_server_is_as_good_as_no_answer() {
    let (url, _received) = one_request_answered("500 Internal Server Error", "");

    let error = HttpTriphaseServer::default()
        .post(&url, &[("op", "pre".to_owned())])
        .expect_err("el servidor fallo");

    assert_eq!(error.situation(), Situation::ServerUnreachable);
}

#[test]
fn a_server_that_is_not_listening_is_unreachable() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("hay un puerto libre");
    let url = format!(
        "http://{}/tri",
        listener.local_addr().expect("tiene direccion")
    );
    drop(listener);

    let error = HttpTriphaseServer::default()
        .post(&url, &[("op", "pre".to_owned())])
        .expect_err("nadie escucha");

    assert_eq!(error.situation(), Situation::ServerUnreachable);
}

#[test]
fn the_url_credentials_travel_as_a_basic_authorization_header() {
    use crate::site::adapters::header_probe::{authorization_probe, with_alice, ALICE_BASIC};
    let (address, received) = authorization_probe();
    let url = with_alice(&address);

    let _ = HttpTriphaseServer::default().post(&url, &[("op", "pre".to_owned())]);

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

    let _ = HttpTriphaseServer::default().post(&url, &[("op", "pre".to_owned())]);

    assert_eq!(received.recv().expect("llego la peticion"), None);
}

#[test]
fn a_rejection_of_the_server_keeps_the_body_of_the_error() {
    let url = crate::site::adapters::header_probe::rejecting_server(
        "404 Not Found",
        "ERR-01: documento desconocido",
    );

    let error = HttpTriphaseServer::default()
        .post(&url, &[("op", "pre".to_owned())])
        .expect_err("el servidor rechaza");

    assert!(
        error.to_string().contains("ERR-01: documento desconocido"),
        "{error}"
    );
}

#[test]
fn an_answer_after_the_connect_limit_still_arrives() {
    use crate::site::adapters::header_probe::{slow_server, BEYOND_CONNECT_LIMIT};
    let url = slow_server(BEYOND_CONNECT_LIMIT, "contenido");

    let answer = HttpTriphaseServer::default()
        .post(&url, &[])
        .expect("el servidor contesta");

    assert_eq!(answer, b"contenido");
}

#[test]
fn an_untrusted_server_certificate_is_named_as_such_and_stays_unreachable() {
    let url = crate::site::adapters::header_probe::untrusted_tls_server();

    let error = HttpTriphaseServer::default()
        .post(&url, &[])
        .expect_err("el certificado no es de confianza");

    assert_eq!(error.situation(), Situation::ServerUnreachable);
    assert!(
        error
            .to_string()
            .contains("certificado del servidor 127.0.0.1 no es de confianza"),
        "{error}"
    );
}
