use super::*;

/// El eco tal y como lo manda el `autoscript.js` publicado
/// (`autoscript.js:2286`).
const PUBLISHED_ECHO: &str = "echo=-idsession=8jAkPZfRw2mQxN4TbYuL@EOF";

#[test]
fn the_echo_the_published_client_sends_is_read_whole() {
    let message = ChannelMessage::read(PUBLISHED_ECHO);

    assert_eq!(
        message,
        ChannelMessage::Echo {
            credential: Some("8jAkPZfRw2mQxN4TbYuL".to_owned()),
        }
    );
    assert_eq!(message.credential(), Some("8jAkPZfRw2mQxN4TbYuL"));
}

#[test]
fn an_echo_without_the_end_marker_is_still_an_echo() {
    let message = ChannelMessage::read("echo=-idsession=8jAkPZfRw2mQxN4TbYuL");

    assert_eq!(message.credential(), Some("8jAkPZfRw2mQxN4TbYuL"));
}

#[test]
fn an_echo_that_forgot_the_credential_carries_none() {
    let message = ChannelMessage::read("echo=@EOF");

    assert_eq!(message, ChannelMessage::Echo { credential: None });
    assert_eq!(message.credential(), None);
}

#[test]
fn an_operation_repeats_the_credential_in_its_own_parameter() {
    let message = ChannelMessage::read(
        "afirma://sign?op=sign&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES",
    );

    assert_eq!(message.credential(), Some("8jAkPZfRw2mQxN4TbYuL"));
    let ChannelMessage::Operation { url } = &message else {
        panic!("una URL del protocolo es una operacion");
    };
    assert_eq!(url.verb(), "sign");
}

#[test]
fn a_credential_that_ends_in_the_marker_is_trimmed_like_in_the_original() {
    let message = ChannelMessage::read("afirma://sign?idsession=8jAkPZfRw2mQxN4TbYuL@EOF");

    assert_eq!(message.credential(), Some("8jAkPZfRw2mQxN4TbYuL"));
}

#[test]
fn anything_that_is_neither_an_echo_nor_a_protocol_url_is_not_of_the_protocol() {
    assert_eq!(
        ChannelMessage::read("GET / HTTP/1.1"),
        ChannelMessage::NotOfTheProtocol
    );
    assert_eq!(
        ChannelMessage::read("https://sede.example/firmar"),
        ChannelMessage::NotOfTheProtocol
    );
    assert_eq!(ChannelMessage::read("").credential(), None);
}

#[test]
fn surrounding_whitespace_does_not_hide_the_echo() {
    let message = ChannelMessage::read("  echo=-idsession=8jAkPZfRw2mQxN4TbYuL@EOF\n");

    assert_eq!(message.credential(), Some("8jAkPZfRw2mQxN4TbYuL"));
}
