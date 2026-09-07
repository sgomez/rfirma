use super::*;

use crate::site::domain::protocol::launch::ChannelCredential;

/// El eco tal y como lo manda el `autoscript.js` publicado (`autoscript.js:3081`), idéntico al
/// que manda por WebSocket (`message/tests.rs::PUBLISHED_ECHO`).
const PUBLISHED_ECHO: &str = "echo=-idsession=8jAkPZfRw2mQxN4TbYuL@EOF";

/// Una operación de firma sin fragmentar, tal como la codifica `cmd=` (`autoscript.js:3161`).
const OPERATION_URL: &str = "afirma://sign?op=sign&format=PAdES";

fn cmd_of(url: &str) -> String {
    format!("cmd={}", URL_SAFE.encode(url))
}

#[test]
fn reads_the_published_echo_whole() {
    let request = read_request(PUBLISHED_ECHO).unwrap();

    assert_eq!(
        request,
        FramedRequest::Echo {
            message: PUBLISHED_ECHO.to_owned(),
            resets: true,
        }
    );
}

#[test]
fn an_echo_without_the_reset_marker_does_not_reset() {
    let request = read_request("echo=idsession=8jAkPZfRw2mQxN4TbYuL@EOF").unwrap();

    let FramedRequest::Echo { resets, .. } = request else {
        panic!("un 'echo=' es un eco");
    };
    assert!(!resets);
}

#[test]
fn an_echo_without_credential_carries_none() {
    let request = read_request("echo=-@EOF").unwrap();

    assert_eq!(
        request,
        FramedRequest::Echo {
            message: "echo=@EOF".to_owned(),
            resets: true,
        }
    );
}

#[test]
fn a_command_decodes_its_base64_operation_and_carries_the_credential_as_a_parameter() {
    let raw = format!(
        "{}idsession=8jAkPZfRw2mQxN4TbYuL@EOF",
        cmd_of(OPERATION_URL)
    );

    let request = read_request(&raw).unwrap();

    assert_eq!(
        request,
        FramedRequest::Command {
            message: "afirma://sign?op=sign&format=PAdES&idsession=8jAkPZfRw2mQxN4TbYuL@EOF"
                .to_owned(),
        }
    );
}

#[test]
fn a_command_without_credential_is_handed_over_unchanged() {
    let raw = cmd_of(OPERATION_URL);

    let request = read_request(&raw).unwrap();

    assert_eq!(
        request,
        FramedRequest::Command {
            message: OPERATION_URL.to_owned(),
        }
    );
}

#[test]
fn a_command_that_is_not_base64_is_not_of_the_framing() {
    assert_eq!(read_request("cmd=not-base64!!!"), Err(NotOfTheFraming));
}

#[test]
fn a_fragment_reads_its_position_total_and_decoded_chunk() {
    let chunk = URL_SAFE.encode("afirma://sign?op=sig");
    let raw = format!("fragment=@1@3@{chunk}idsession=8jAkPZfRw2mQxN4TbYuL@EOF");

    let request = read_request(&raw).unwrap();

    assert_eq!(
        request,
        FramedRequest::Fragment {
            part: 1,
            total: 3,
            chunk: "afirma://sign?op=sig".to_owned(),
            credential: Some("8jAkPZfRw2mQxN4TbYuL".to_owned()),
        }
    );
}

#[test]
fn a_firm_carries_only_the_credential() {
    let request = read_request("firm=idsession=8jAkPZfRw2mQxN4TbYuL@EOF").unwrap();

    assert_eq!(
        request,
        FramedRequest::Firm {
            credential: Some("8jAkPZfRw2mQxN4TbYuL".to_owned()),
        }
    );
}

#[test]
fn a_send_reads_the_requested_part_and_total() {
    let request = read_request("send=@2@3idsession=8jAkPZfRw2mQxN4TbYuL@EOF").unwrap();

    assert_eq!(
        request,
        FramedRequest::Send {
            part: 2,
            total: 3,
            credential: Some("8jAkPZfRw2mQxN4TbYuL".to_owned()),
        }
    );
}

#[test]
fn anything_without_one_of_the_five_commands_is_not_of_the_framing() {
    assert_eq!(read_request("GET / HTTP/1.1"), Err(NotOfTheFraming));
    assert_eq!(read_request(""), Err(NotOfTheFraming));
}

#[test]
fn a_required_credential_that_matches_is_accepted() {
    let negotiated =
        NegotiatedCredential::Required(ChannelCredential::parse("8jAkPZfRw2mQxN4TbYuL").unwrap());

    assert!(credential_matches(
        &negotiated,
        Some("8jAkPZfRw2mQxN4TbYuL")
    ));
}

#[test]
fn a_required_credential_that_does_not_match_is_refused() {
    let negotiated =
        NegotiatedCredential::Required(ChannelCredential::parse("8jAkPZfRw2mQxN4TbYuL").unwrap());

    assert!(!credential_matches(&negotiated, Some("otra")));
    assert!(!credential_matches(&negotiated, None));
}

#[test]
fn an_absent_negotiation_accepts_anything() {
    assert!(credential_matches(&NegotiatedCredential::Absent, None));
    assert!(credential_matches(
        &NegotiatedCredential::Absent,
        Some("lo que sea")
    ));
}

#[test]
fn a_fragment_buffer_reassembles_parts_received_in_order() {
    let mut buffer = FragmentBuffer::new();

    buffer.insert(1, "afirma://sign?".to_owned());
    buffer.insert(2, "op=sign&".to_owned());
    buffer.insert(3, "format=PAdES".to_owned());

    assert_eq!(
        buffer.combined().as_deref(),
        Some("afirma://sign?op=sign&format=PAdES")
    );
}

#[test]
fn a_fragment_buffer_replaces_a_part_sent_again() {
    let mut buffer = FragmentBuffer::new();

    buffer.insert(1, "primera".to_owned());
    buffer.insert(1, "definitiva".to_owned());

    assert_eq!(buffer.combined().as_deref(), Some("definitiva"));
}

#[test]
fn an_empty_fragment_buffer_has_nothing_combined() {
    assert_eq!(FragmentBuffer::new().combined(), None);
}

#[test]
fn reset_discards_what_was_reassembled() {
    let mut buffer = FragmentBuffer::new();
    buffer.insert(1, "algo".to_owned());

    buffer.reset();

    assert_eq!(buffer.combined(), None);
}

#[test]
fn a_short_response_is_a_single_part() {
    assert_eq!(split_response("SAF_02:mensaje"), vec!["SAF_02:mensaje"]);
}

#[test]
fn an_empty_response_has_no_parts() {
    assert_eq!(split_response(""), Vec::<String>::new());
}

#[test]
fn a_long_response_is_split_at_the_maximum_part_size() {
    let text = "a".repeat(RESPONSE_MAX_SIZE + 1);

    let parts = split_response(&text);

    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].len(), RESPONSE_MAX_SIZE);
    assert_eq!(parts[1].len(), 1);
    assert_eq!(parts.concat(), text);
}

/// Byte a byte contra `createHttpResponse(true, "OK")` (línea 493): las cinco cabeceras
/// terminadas en `\n`, la línea en blanco y el cuerpo en Base64 URL-safe.
#[test]
fn the_http_response_matches_the_original_byte_for_byte() {
    let response = http_response("OK");

    assert_eq!(
        response,
        b"HTTP/1.1 200 OK\n\
Connection: close\n\
Pragma: no-cache\n\
Server: Cliente @firma\n\
Content-Type: text/html; charset=utf-8\n\
Access-Control-Allow-Origin: *\n\
\n\
T0s="
    );
}

#[test]
fn the_http_response_body_decodes_back_to_the_original_text() {
    let response = String::from_utf8(http_response(OPERATION_URL)).unwrap();
    let encoded = response.rsplit("\n\n").next().unwrap();

    let decoded = URL_SAFE.decode(encoded).unwrap();

    assert_eq!(String::from_utf8(decoded).unwrap(), OPERATION_URL);
}
