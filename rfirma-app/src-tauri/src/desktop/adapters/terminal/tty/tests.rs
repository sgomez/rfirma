use std::io::{self, Read};

use super::typed_line;

struct BrokenTerminal;

impl Read for BrokenTerminal {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("EIO"))
    }
}

#[test]
fn the_line_ends_at_the_newline_and_leaves_the_rest_unread() {
    let mut input: &[u8] = b"1234\nnext";

    let secret = typed_line(&mut input).unwrap();

    assert_eq!(secret.as_bytes(), b"1234");
    assert_eq!(input, b"next");
}

#[test]
fn a_carriage_return_before_the_newline_is_not_part_of_the_secret() {
    let mut input: &[u8] = b"1234\r\n";

    assert_eq!(typed_line(&mut input).unwrap().as_bytes(), b"1234");
}

#[test]
fn an_empty_line_is_an_empty_secret() {
    let mut input: &[u8] = b"\n";

    assert!(typed_line(&mut input).unwrap().is_empty());
}

#[test]
fn a_terminal_closed_before_the_newline_is_refused() {
    let mut input: &[u8] = b"12";

    assert_eq!(
        typed_line(&mut input).unwrap_err(),
        "no se ha tecleado nada"
    );
}

#[test]
fn a_terminal_that_fails_to_read_names_the_system_error() {
    let error = typed_line(&mut BrokenTerminal).unwrap_err();

    assert!(error.contains("EIO"), "{error}");
}
