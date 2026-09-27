use super::{bmp_string, module_spec, store_pin};
use crate::identity::domain::error::Situation;
use crate::identity::domain::protected_secret::ProtectedSecret;
use std::path::Path;

#[test]
fn the_password_travels_as_a_big_endian_bmp_string_with_its_terminator() {
    assert_eq!(
        bmp_string("1234"),
        vec![0, b'1', 0, b'2', 0, b'3', 0, b'4', 0, 0]
    );
}

#[test]
fn a_password_outside_ascii_keeps_the_big_endian_order() {
    assert_eq!(bmp_string("ñ"), vec![0x00, 0xf1, 0, 0]);
}

#[test]
fn an_empty_password_is_just_the_terminator() {
    assert_eq!(bmp_string(""), vec![0, 0]);
}

#[test]
fn the_store_is_created_in_sql_format_and_writable() {
    let spec = module_spec(Path::new("/casa/datos/rfirma/certificates/abc"));

    assert!(spec.contains("configDir='sql:/casa/datos/rfirma/certificates/abc'"));
    assert!(spec.contains("flags=readWrite"));
}

#[test]
fn a_utf8_pin_becomes_the_c_string_nss_expects() {
    let pin = store_pin(&ProtectedSecret::new("1234")).unwrap();

    assert_eq!(pin.as_bytes(), b"1234");
}

#[test]
fn a_pin_that_is_not_utf8_is_an_incorrect_pin() {
    let refused = store_pin(&ProtectedSecret::new([0xff, 0xfe])).unwrap_err();

    assert_eq!(refused.situation(), Situation::IncorrectPin);
}

#[test]
fn a_pin_with_a_nul_inside_is_refused() {
    assert!(store_pin(&ProtectedSecret::new("12\u{0}34")).is_err());
}
