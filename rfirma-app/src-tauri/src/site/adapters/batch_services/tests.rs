use super::*;

#[test]
fn a_malformed_batch_url_fails_without_panicking() {
    assert!(parsed_batch_url("no es una url", Situation::PostsignerUnreachable).is_err());
}

#[test]
fn presign_composes_the_format_the_lote_and_the_certs_without_tridata() {
    let query = compose_query(
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

    let json_query = compose_query(BatchFormat::Json, "bG90ZQ", &[], Some(&tridata));
    assert!(json_query.ends_with(&format!("&tridata={}", URL_SAFE.encode(tridata.to_json()))));

    let xml_query = compose_query(BatchFormat::Xml, "bG90ZQ", &[], Some(&tridata));
    assert!(xml_query.ends_with(&format!("&tridata={}", URL_SAFE.encode(tridata.to_xml()))));
}

#[test]
fn json_uses_the_json_parameter_name() {
    let query = compose_query(BatchFormat::Json, "bG90ZQ", &[], None);

    assert!(query.starts_with("json=bG90ZQ&certs="));
}

#[test]
fn the_lote_gets_the_textual_substitution_instead_of_being_reencoded() {
    let query = compose_query(BatchFormat::Xml, "a+b/c+d/e", &[], None);

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
