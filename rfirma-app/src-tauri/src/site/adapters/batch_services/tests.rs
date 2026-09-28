use super::*;

#[test]
fn a_malformed_batch_url_fails_without_panicking() {
    assert!(parsed_batch_url("no es una url", Situation::PostsignerUnreachable).is_err());
}

#[test]
fn presign_composes_the_format_the_lote_and_the_certs_without_tridata() {
    let query = compose_body(
        BatchFormat::Xml,
        "bG90ZQ",
        &[b"cert-uno".to_vec(), b"cert-dos".to_vec()],
        None,
    );

    assert_eq!(
        query,
        format!(
            "xml=bG90ZQ&certs={}",
            [URL_SAFE.encode(b"cert-uno"), URL_SAFE.encode(b"cert-dos")].join(";")
        )
    );
}

#[test]
fn postsign_appends_the_tridata_url_safe_and_in_the_lotes_own_format() {
    let tridata = TriphaseData::new(
        None,
        vec![crate::site::domain::batch::TriSign::new(
            Some("001".to_owned()),
            None,
            vec![("PK1".to_owned(), "AAAA".to_owned())],
        )],
    );

    let json_query = compose_body(BatchFormat::Json, "bG90ZQ", &[], Some(&tridata));
    assert!(json_query.ends_with(&format!("&tridata={}", URL_SAFE.encode(tridata.to_json()))));

    let xml_query = compose_body(BatchFormat::Xml, "bG90ZQ", &[], Some(&tridata));
    assert!(xml_query.ends_with(&format!("&tridata={}", URL_SAFE.encode(tridata.to_xml()))));
}

#[test]
fn json_uses_the_json_parameter_name() {
    let query = compose_body(BatchFormat::Json, "bG90ZQ", &[], None);

    assert!(query.starts_with("json=bG90ZQ&certs="));
}

#[test]
fn the_lote_gets_the_textual_substitution_instead_of_being_reencoded() {
    let query = compose_body(BatchFormat::Xml, "a+b/c+d/e", &[], None);

    assert!(query.starts_with("xml=a-b_c-d_e&certs="));
}

#[tokio::test]
async fn presign_does_not_panic_inside_tokio_context() {
    let services = std::thread::spawn(RelayBatchServices::default)
        .join()
        .unwrap();
    let result = services.presign("https://example.com/pre", BatchFormat::Json, "bG90ZQ", &[]);
    assert!(result.is_err());
    std::thread::spawn(move || drop(services)).join().unwrap();
}

fn servlet_answering(status: &'static str) -> String {
    use std::io::{BufRead, BufReader, Write};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("hay un puerto libre");
    let url = format!(
        "http://{}/servlet",
        listener.local_addr().expect("tiene direccion")
    );
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("llega la peticion");
        let mut reader = BufReader::new(stream);
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("se lee la cabecera");
            if line == "\r\n" || line.is_empty() {
                break;
            }
        }
        let answer = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        reader
            .get_mut()
            .write_all(answer.as_bytes())
            .expect("se contesta");
    });
    url
}

#[test]
fn a_presigner_rejection_keeps_its_http_status() {
    let url = servlet_answering("400 Bad Request");

    let error = RelayBatchServices::default()
        .presign(&url, BatchFormat::Json, "bG90ZQ", &[])
        .expect_err("el servlet rechaza");

    assert_eq!(error.situation(), Situation::InvalidPresignResponse);
    assert_eq!(error.http_status(), Some(400));
}

#[test]
fn a_postsigner_rejection_keeps_its_http_status() {
    let url = servlet_answering("503 Service Unavailable");
    let tridata = TriphaseData::new(None, vec![]);

    let error = RelayBatchServices::default()
        .postsign(&url, BatchFormat::Json, "bG90ZQ", &[], &tridata)
        .expect_err("el servlet rechaza");

    assert_eq!(error.situation(), Situation::InvalidPostsignResponse);
    assert_eq!(error.http_status(), Some(503));
}

struct ServletRequest {
    target: String,
    content_type: Option<String>,
    body: String,
}

fn servlet_recording() -> (String, std::sync::mpsc::Receiver<ServletRequest>) {
    use std::io::{BufRead, BufReader, Read, Write};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("hay un puerto libre");
    let url = format!(
        "http://{}/servlet",
        listener.local_addr().expect("tiene direccion")
    );
    let (sender, received) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("llega la peticion");
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        reader
            .read_line(&mut request_line)
            .expect("se lee la peticion");
        let mut content_type = None;
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("se lee la cabecera");
            if line == "\r\n" || line.is_empty() {
                break;
            }
            let (name, value) = line.split_once(':').unwrap_or((&line, ""));
            match name.to_ascii_lowercase().as_str() {
                "content-type" => content_type = Some(value.trim().to_owned()),
                "content-length" => content_length = value.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).expect("se lee el cuerpo");
        reader
            .get_mut()
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .expect("se contesta");
        let target = request_line
            .split(' ')
            .nth(1)
            .unwrap_or_default()
            .to_owned();
        let body = String::from_utf8(body).expect("el cuerpo es texto");
        sender
            .send(ServletRequest {
                target,
                content_type,
                body,
            })
            .expect("la prueba espera la peticion");
    });
    (url, received)
}

#[test]
fn presign_sends_its_parameters_as_a_form_in_the_post_body() {
    let (url, received) = servlet_recording();
    let certs = [b"cert-uno".to_vec()];

    RelayBatchServices::default()
        .presign(&url, BatchFormat::Json, "bG90ZQ", &certs)
        .expect("el servlet atiende");

    let request = received.recv().expect("llego la peticion");
    assert_eq!(request.target, "/servlet");
    assert_eq!(
        request.content_type.as_deref(),
        Some("application/x-www-form-urlencoded")
    );
    assert_eq!(
        request.body,
        compose_body(BatchFormat::Json, "bG90ZQ", &certs, None)
    );
}

#[test]
fn postsign_sends_its_parameters_and_the_tridata_in_the_post_body() {
    let (url, received) = servlet_recording();
    let tridata = TriphaseData::new(None, vec![]);

    RelayBatchServices::default()
        .postsign(&url, BatchFormat::Json, "bG90ZQ", &[], &tridata)
        .expect("el servlet atiende");

    let request = received.recv().expect("llego la peticion");
    assert_eq!(request.target, "/servlet");
    assert_eq!(
        request.body,
        compose_body(BatchFormat::Json, "bG90ZQ", &[], Some(&tridata))
    );
}

#[test]
fn the_url_credentials_travel_as_a_basic_authorization_header() {
    use crate::site::adapters::header_probe::{authorization_probe, with_alice, ALICE_BASIC};
    let (address, received) = authorization_probe();
    let url = with_alice(&address);

    let _ = RelayBatchServices::default().presign(&url, BatchFormat::Json, "bG90ZQ", &[]);

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

    let _ = RelayBatchServices::default().presign(&url, BatchFormat::Json, "bG90ZQ", &[]);

    assert_eq!(received.recv().expect("llego la peticion"), None);
}
