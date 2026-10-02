use std::io::{self, Read};

use super::first_line;

struct BrokenDescriptor;

impl Read for BrokenDescriptor {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("EIO"))
    }
}

#[test]
fn the_secret_is_the_first_line_without_its_newline() {
    let mut input: &[u8] = b"1234\nlo demas";

    assert_eq!(first_line(&mut input).unwrap().as_bytes(), b"1234");
}

#[test]
fn a_secret_without_a_newline_is_whole() {
    let mut input: &[u8] = b"1234";

    assert_eq!(first_line(&mut input).unwrap().as_bytes(), b"1234");
}

#[test]
fn a_carriage_return_before_the_newline_is_not_part_of_the_secret() {
    let mut input: &[u8] = b"1234\r\n";

    assert_eq!(first_line(&mut input).unwrap().as_bytes(), b"1234");
}

#[test]
fn an_empty_descriptor_is_refused() {
    let mut input: &[u8] = b"";
    let mut only_a_newline: &[u8] = b"\n";

    assert!(first_line(&mut input).is_err());
    assert!(first_line(&mut only_a_newline).is_err());
}

#[test]
fn a_descriptor_that_fails_to_read_is_refused() {
    assert!(first_line(&mut BrokenDescriptor)
        .unwrap_err()
        .contains("EIO"));
}

#[cfg(unix)]
#[test]
fn a_descriptor_that_is_not_open_is_refused() {
    assert!(super::read_from(9999)
        .unwrap_err()
        .contains("no está abierto"));
}
